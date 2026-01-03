// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use super::super::Error;
use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("run", "Run a Simplicity program in the context of a PSET input.")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true),
			cmd::arg("input-index", "the index of the input to sign (decimal)")
				.takes_value(true)
				.required(true),
			cmd::arg("program", "Simplicity program (base64)").takes_value(true).required(true),
			cmd::arg("witness", "Simplicity program witness (hex)")
				.takes_value(true)
				.required(true),
			cmd::opt(
				"genesis-hash",
				"genesis hash of the blockchain the transaction belongs to (hex)",
			)
			.short("g")
			.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory");
	let input_idx = matches.value_of("input-index").expect("input-idx is mandatory");
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness").expect("witness is mandatory");
	let genesis_hash = matches.value_of("genesis-hash");

	match hal_simplicity::actions::simplicity::pset::pset_run(
		pset_b64,
		input_idx,
		program,
		witness,
		genesis_hash,
	) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
