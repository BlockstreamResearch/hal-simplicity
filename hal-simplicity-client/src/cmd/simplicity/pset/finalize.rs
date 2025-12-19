// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

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
		])
}

pub fn exec<'a>(
	matches: &clap::ArgMatches<'a>,
	client: &hal_simplicity::hal_simplicity_client::HalSimplicity,
) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory").to_string();
	let input_idx_str = matches.value_of("input-index").expect("input-idx is mandatory");
	let input_idx: u32 = input_idx_str.parse().expect("invalid input index");
	let program = matches.value_of("program").expect("program is mandatory").to_string();
	let witness = matches.value_of("witness").expect("witness is mandatory").to_string();
	let genesis_hash = matches.value_of("genesis-hash").map(String::from);

	match client.pset_finalize(pset_b64, input_idx, program, witness, genesis_hash) {
		Ok(response) => cmd::print_output(matches, &response),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
