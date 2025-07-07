use std::panic;
use std::process;

pub use elements::bitcoin;

pub use hal_simplicity::{GetInfo, Network};

pub mod cmd;

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
		.bin_name("hal")
		.version(clap::crate_version!())
		.subcommand(
			cmd::subcommand_group("simplicity", "Simplicity extensions for hal")
				.about("hal-simplicity -- a Simplicity extension of hal")
				.setting(clap::AppSettings::GlobalVersion)
				.setting(clap::AppSettings::VersionlessSubcommands)
				.setting(clap::AppSettings::SubcommandRequiredElseHelp)
				.setting(clap::AppSettings::DisableHelpSubcommand)
				.setting(clap::AppSettings::AllArgsOverrideSelf)
				.subcommands(cmd::subcommands()),
		)
		.setting(clap::AppSettings::SubcommandRequiredElseHelp)
		.arg(
			cmd::opt("verbose", "print verbose logging output to stderr")
				.short("v")
				.takes_value(false)
				.global(true),
		)
}

/// Try execute built-in command. Return false if no command found.
fn execute_builtin<'a>(matches: &clap::ArgMatches<'a>) -> bool {
	match matches.subcommand() {
		("address", Some(m)) => cmd::address::execute(m),
		("block", Some(m)) => cmd::block::execute(m),
		("keypair", Some(m)) => cmd::keypair::execute(m),
		("tx", Some(m)) => cmd::tx::execute(m),
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

	match matches.subcommand() {
		("simplicity", Some(m)) => {
			if execute_builtin(m) {
				// success
				process::exit(0);
			} else {
				panic!("Subcommand not found: {}", m.subcommand().0);
			}
		}
		(cmd, _) => panic!("Subcommand not found: {:?}", cmd),
	}
}
