use clap;

use crate::cmd;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

use hal_simplicity_daemon::Network;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("address", "work with addresses")
		.subcommand(cmd_create())
		.subcommand(cmd_inspect())
}

pub fn execute(matches: &clap::ArgMatches<'_>, client: &HalSimplicity) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m, client),
		("inspect", Some(m)) => exec_inspect(m, client),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_create<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create addresses").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::opt("pubkey", "a public key in hex").takes_value(true).required(false),
		cmd::opt("script", "a script in hex").takes_value(true).required(false),
		cmd::opt("blinder", "a blinding pubkey in hex").takes_value(true).required(false),
	])
}

fn exec_create(matches: &clap::ArgMatches<'_>, client: &HalSimplicity) {
	let network = cmd::network(matches);

	let network_str = match network {
		Network::ElementsRegtest => Some("elementsregtest".to_string()),
		Network::Liquid => Some("liquid".to_string()),
		Network::LiquidTestnet => Some("liquidtestnet".to_string()),
	};

	let pubkey = matches.value_of("pubkey").map(|s| s.to_string());
	let script = matches.value_of("script").map(|s| s.to_string());
	let blinder = matches.value_of("blinder").map(|s| s.to_string());

	if pubkey.is_none() && script.is_none() {
		panic!("Can't create addresses without a pubkey or script");
	}

	let result = client
		.address_create(network_str, pubkey, script, blinder)
		.expect("failed to create address");

	cmd::print_output(matches, &result);
}

fn cmd_inspect<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("inspect", "inspect addresses")
		.args(&[cmd::opt_yaml(), cmd::arg("address", "the address").required(true)])
}

fn exec_inspect(matches: &clap::ArgMatches<'_>, client: &HalSimplicity) {
	let address_str = matches.value_of("address").expect("no address provided");

	let result =
		client.address_inspect(address_str.to_string()).expect("failed to inspect address");

	cmd::print_output(matches, &result);
}
