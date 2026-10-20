// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use super::super::Error;
use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("finalize", "Attach a Simplicity program and witness to a PSET input")
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
			cmd::opt(
				"padding-bytes",
				"size of the annex to pad the witness with so the program's cost fits its budget, including the 0x50 prefix byte (decimal)",
			)
			.short("a")
			.alias("padding")
			.takes_value(true)
			.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory");
	let input_idx = matches.value_of("input-index").expect("input-idx is mandatory");
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness").expect("witness is mandatory");
	let genesis_hash = matches.value_of("genesis-hash");
	let padding_bytes = match matches.value_of("padding-bytes").map(str::parse).transpose() {
		Ok(n) => n.unwrap_or(0),
		Err(e) => {
			return cmd::print_output(
				matches,
				&Error {
					error: format!("invalid padding-bytes: {}", e),
				},
			)
		}
	};

	match hal_simplicity::actions::simplicity::pset::pset_finalize(
		pset_b64,
		input_idx,
		program,
		witness,
		padding_bytes,
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
