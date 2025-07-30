use clap;
use elements::bitcoin::secp256k1::{self, rand};

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
	#[derive(serde::Serialize)]
	struct Res {
		secret: secp256k1::SecretKey,
		x_only: secp256k1::XOnlyPublicKey,
		parity: secp256k1::Parity,
	}

	let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
	let (x_only, parity) = public.x_only_public_key();

	cmd::print_output(
		matches,
		&Res {
			secret,
			x_only,
			parity,
		},
	);
}
