use clap;

use crate::cmd;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("keypair", "manipulate private and public keys")
		.subcommand(cmd_generate())
}

pub fn execute(matches: &clap::ArgMatches<'_>, client: &HalSimplicity) {
	match matches.subcommand() {
		("generate", Some(m)) => exec_generate(m, client),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_generate<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("generate", "generate a random private/public keypair").args(&[cmd::opt_yaml()])
}

fn exec_generate(matches: &clap::ArgMatches<'_>, client: &HalSimplicity) {
	let result = client.keypair_generate().expect("failed to generate keypair");
	cmd::print_output(matches, &result);
}
