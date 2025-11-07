pub mod address;
pub mod block;
pub mod keypair;
pub mod simplicity;
pub mod tx;

use std::borrow::Cow;
use std::io;
use std::io::Read;
use std::fmt;

use hal_simplicity::Network;

/// Error type for command execution
#[derive(Debug)]
pub enum CmdError {
	Serialization(String),
	InvalidInput(String),
	ParseError(String),
	IoError(String),
}

impl fmt::Display for CmdError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			CmdError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
			CmdError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
			CmdError::ParseError(msg) => write!(f, "Parse error: {}", msg),
			CmdError::IoError(msg) => write!(f, "IO error: {}", msg),
		}
	}
}

impl std::error::Error for CmdError {}

/// Build a list of all built-in subcommands.
pub fn subcommands<'a>() -> Vec<clap::App<'a, 'a>> {
	vec![
		address::subcommand(),
		block::subcommand(),
		keypair::subcommand(),
		simplicity::subcommand(),
		tx::subcommand(),
	]
}

/// Construct a new command option.
pub fn opt<'a>(name: &'static str, help: &'static str) -> clap::Arg<'a, 'a> {
	clap::Arg::with_name(name).long(name).help(help)
}

/// Construct a new positional argument.
pub fn arg<'a>(name: &'static str, help: &'static str) -> clap::Arg<'a, 'a> {
	clap::Arg::with_name(name).help(help).takes_value(true)
}

/// Create a new subcommand group using the template that sets all the common settings.
/// This is not intended for actual commands, but for subcommands that host a bunch of other
/// subcommands.
pub fn subcommand_group<'a>(name: &'static str, about: &'static str) -> clap::App<'a, 'a> {
	clap::SubCommand::with_name(name)
		.about(about)
		.setting(clap::AppSettings::SubcommandRequiredElseHelp)
		//.setting(clap::AppSettings::AllowExternalSubcommands)
		.setting(clap::AppSettings::DisableHelpSubcommand)
		.setting(clap::AppSettings::VersionlessSubcommands)
}

/// Create a new subcommand using the template that sets all the common settings.
pub fn subcommand<'a>(name: &'static str, about: &'static str) -> clap::App<'a, 'a> {
	clap::SubCommand::with_name(name).about(about).setting(clap::AppSettings::DisableHelpSubcommand)
}

pub fn opts_networks<'a>() -> Vec<clap::Arg<'a, 'a>> {
	vec![
		clap::Arg::with_name("elementsregtest")
			.long("elementsregtest")
			.short("r")
			.help("run in elementsregtest mode")
			.takes_value(false)
			.required(false),
		clap::Arg::with_name("liquid")
			.long("liquid")
			.help("run in liquid mode")
			.takes_value(false)
			.required(false),
	]
}

pub fn network<'a>(matches: &clap::ArgMatches<'a>) -> Network {
	if matches.is_present("elementsregtest") {
		Network::ElementsRegtest
	} else if matches.is_present("liquid") {
		Network::Liquid
	} else {
		Network::ElementsRegtest
	}
}

pub fn opt_yaml<'a>() -> clap::Arg<'a, 'a> {
	clap::Arg::with_name("yaml")
		.long("yaml")
		.short("y")
		.help("print output in YAML instead of JSON")
		.takes_value(false)
		.required(false)
}

/// Get the named argument from the CLI arguments or try read from stdin if not provided.
pub fn arg_or_stdin<'a>(matches: &'a clap::ArgMatches<'a>, arg: &str) -> Cow<'a, str> {
	if let Some(s) = matches.value_of(arg) {
		s.into()
	} else {
		// Read from stdin.
		let mut input = Vec::new();
		let stdin = io::stdin();
		let mut stdin_lock = stdin.lock();
		let _ = stdin_lock.read_to_end(&mut input);
		while stdin_lock.read_to_end(&mut input).unwrap_or(0) > 0 {}
		if input.is_empty() {
			panic!("no '{}' argument given", arg);
		}
		String::from_utf8(input)
			.unwrap_or_else(|e| panic!("invalid utf8 on stdin for '{}': {}", arg, e))
			.trim()
			.to_owned()
			.into()
	}
}

/// Serialize output to String (for library use)
/// This allows functions to return data instead of just printing
/// Returns Result for proper error handling in production
pub fn serialize_output<'a, T: serde::Serialize>(
	matches: &clap::ArgMatches<'a>,
	out: &T,
) -> Result<String, CmdError> {
	if matches.is_present("yaml") {
		serde_yaml::to_string(&out)
			.map_err(|e| CmdError::Serialization(e.to_string()))
	} else {
		serde_json::to_string_pretty(&out)
			.map_err(|e| CmdError::Serialization(e.to_string()))
	}
}

/// Print output to stdout (for CLI use)
/// Now calls serialize_output and prints the result
pub fn print_output<'a, T: serde::Serialize>(matches: &clap::ArgMatches<'a>, out: &T) {
	match serialize_output(matches, out) {
		Ok(output) => println!("{}", output),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
