use clap;

use crate::cmd;

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("keypair", "manipulate private and public keys")
		.subcommand(cmd_generate())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("generate", Some(m)) => exec_generate(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_generate<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("generate", "generate a random private/public keypair").args(&[cmd::opt_yaml()])
}

fn exec_generate<'a>(matches: &clap::ArgMatches<'a>) {
	let keypair = hal_simplicity::actions::keypair::keypair_generate();
	cmd::print_output(matches, &keypair);
}
