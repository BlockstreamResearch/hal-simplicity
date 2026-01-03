use clap;

use crate::cmd;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("address", "work with addresses")
		.subcommand(cmd_create())
		.subcommand(cmd_inspect())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m),
		("inspect", Some(m)) => exec_inspect(m),
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

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let network = cmd::network(matches);
	let pubkey_hex = matches.value_of("pubkey");
	let script_hex = matches.value_of("script");
	let blinder_hex = matches.value_of("blinder");

	match hal_simplicity::actions::address::address_create(
		pubkey_hex,
		script_hex,
		blinder_hex,
		network,
	) {
		Ok(addresses) => cmd::print_output(matches, &addresses),
		Err(e) => panic!("{}", e),
	}
}

fn cmd_inspect<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("inspect", "inspect addresses")
		.args(&[cmd::opt_yaml(), cmd::arg("address", "the address").required(true)])
}

fn exec_inspect<'a>(matches: &clap::ArgMatches<'a>) {
	let address_str = matches.value_of("address").expect("address is required");

	match hal_simplicity::actions::address::address_inspect(address_str) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => panic!("{}", e),
	}
}
