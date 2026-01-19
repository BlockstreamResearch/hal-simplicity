#[cfg(not(feature = "daemon"))]
fn main() {
	eprintln!("hal-simplicity-daemon can only be built with the 'daemon' feature enabled");
	std::process::exit(1);
}

#[cfg(feature = "daemon")]
fn main() {
	use hal_simplicity::daemon::HalSimplicityDaemon;

	/// Default address for the TCP listener
	const DEFAULT_ADDRESS: &str = "127.0.0.1:28579";

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
		clap::App::new("hal-simplicity-daemon")
			.bin_name("hal-simplicity-daemon")
			.version(clap::crate_version!())
			.about("hal-simplicity-daemon -- JSON-RPC daemon for Simplicity operations")
			.arg(
				clap::Arg::with_name("address")
					.short("a")
					.long("address")
					.value_name("ADDRESS")
					.help("TCP address to bind to (default: 127.0.0.1:28579)")
					.takes_value(true),
			)
			.arg(
				clap::Arg::with_name("verbose")
					.short("v")
					.long("verbose")
					.help("Enable verbose logging output to stderr")
					.takes_value(false),
			)
	}

	let app = init_app();
	let matches = app.get_matches();

	// Enable logging in verbose mode.
	match matches.is_present("verbose") {
		true => setup_logger(log::LevelFilter::Debug),
		false => setup_logger(log::LevelFilter::Info),
	}

	// Get the address from command line or use default
	let address = matches.value_of("address").unwrap_or(DEFAULT_ADDRESS);

	log::info!("Starting hal-simplicity-daemon on {}...", address);

	// Create the daemon
	let daemon = match HalSimplicityDaemon::new(address) {
		Ok(d) => d,
		Err(e) => {
			log::error!("Failed to create daemon: {}", e);

			std::process::exit(1);
		}
	};

	// Start the daemon and block
	if let Err(e) = daemon.listen_blocking() {
		log::error!("Daemon error: {}", e);

		std::process::exit(1);
	}
}
