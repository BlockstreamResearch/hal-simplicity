// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::Error;

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

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness");
	let state = matches.value_of("state");

	match hal_simplicity::actions::simplicity::simplicity_info(program, witness, state) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
