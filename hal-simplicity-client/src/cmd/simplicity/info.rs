// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("info", "Parse a base64-encoded Simplicity program and decode it")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::opt_yaml(),
			cmd::arg("program", "a Simplicity program in base64").takes_value(true).required(true),
			cmd::arg("witness", "a hex encoding of all the witness data for the program")
				.takes_value(true)
				.required(false),
			cmd::opt(
				"state",
				"32-byte state commitment to put alongside the program when generating addresess (hex)",
			)
			.takes_value(true)
			.short("s")
			.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let program = matches.value_of("program").expect("program is mandatory").to_string();
	let witness = matches.value_of("witness").map(String::from);
	let state = matches.value_of("state").map(String::from);
	let network = if matches.is_present("liquid") {
		Some("liquid".to_string())
	} else {
		Some("elementsregtest".to_string())
	};

	match client.simplicity_info(program, witness, state, network) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
