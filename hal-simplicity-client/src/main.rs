use std::panic;
use std::process;

pub use elements::bitcoin;

pub use hal_simplicity_daemon::utils::{GetInfo, Network};

pub mod cmd;

use hal_simplicity::hal_simplicity_client::HalSimplicity;

/// Setup logging with the given log level.
fn setup_logger(lvl: log::LevelFilter) {
	fern::Dispatch::new()
		.format(|out, message, _record| out.finish(format_args!("{}", message)))
		.level(lvl)
		.chain(std::io::stderr())
		.apply()
		.expect("error setting up logger");
}

/// Create the main app object.
fn init_app<'a, 'b>() -> clap::App<'a, 'b> {
	clap::App::new("hal-simplicity")
		.bin_name("hal-simplicity")
		.version(clap::crate_version!())
		.about("hal-simplicity -- a Simplicity-enabled fork of hal")
		.setting(clap::AppSettings::GlobalVersion)
		.setting(clap::AppSettings::VersionlessSubcommands)
		.setting(clap::AppSettings::SubcommandRequiredElseHelp)
		.setting(clap::AppSettings::AllArgsOverrideSelf)
		.subcommands(cmd::subcommands())
		.arg(
			cmd::opt("verbose", "print verbose logging output to stderr")
				.short("v")
				.takes_value(false)
				.global(true),
		)
		.arg(cmd::opt("daemon-url", "URL of hal-simplicity-daemon").takes_value(true).global(true))
}

/// Try execute built-in command. Return false if no command found.
fn execute_builtin<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) -> bool {
	match matches.subcommand() {
		("address", Some(m)) => cmd::address::execute(m, client),
		("block", Some(m)) => cmd::block::execute(m, client),
		("keypair", Some(m)) => cmd::keypair::execute(m, client),
		("simplicity", Some(m)) => cmd::simplicity::execute(m, client),
		("tx", Some(m)) => cmd::tx::execute(m, client),
		_ => return false,
	};
	true
}

fn main() {
	// Apply a custom panic hook to print a more user-friendly message
	// in case the execution fails.
	panic::set_hook(Box::new(|info| {
		let message = if let Some(m) = info.payload().downcast_ref::<String>() {
			m
		} else if let Some(m) = info.payload().downcast_ref::<&str>() {
			m
		} else {
			"No error message provided"
		};
		println!("Execution failed: {}", message);
		process::exit(1);
	}));

	let app = init_app();
	let matches = app.get_matches();

	// Enable logging in verbose mode.
	match matches.is_present("verbose") {
		true => setup_logger(log::LevelFilter::Trace),
		false => setup_logger(log::LevelFilter::Warn),
	}

	// Create JSON-RPC client
	let client = if let Some(url) = matches.value_of("daemon-url") {
		HalSimplicity::new(url.to_string())
	} else {
		HalSimplicity::default()
	};

	let client = client.unwrap_or_else(|e| {
		eprintln!("Failed to connect to daemon: {}", e);
		process::exit(1);
	});

	if execute_builtin(&matches, &client) {
		// success
		process::exit(0);
	} else {
		panic!("Subcommand not found: {}", matches.subcommand().0);
	}
}
