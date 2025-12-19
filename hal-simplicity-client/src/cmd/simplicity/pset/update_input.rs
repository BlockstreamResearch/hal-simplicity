// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("update-input", "Attach UTXO data to a PSET input")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true),
			cmd::arg("input-index", "the index of the input to sign (decimal)")
				.takes_value(true)
				.required(true),
			cmd::opt("input-utxo", "the input's UTXO, in the form <scriptPubKey hex>:<asset ID or commitment hex>:<decimal BTC amount or value commitment hex>")
				.short("i")
				.takes_value(true)
				.required(true),
			cmd::opt("internal-key", "internal public key (hex)")
				.short("p")
				.takes_value(true)
				.required(false),
			cmd::opt("cmr", "CMR of the Simplicity program (hex)")
				.short("c")
				.takes_value(true)
				.required(false),
			cmd::opt(
				"state",
				"32-byte state commitment to put alongside the program when generating addresess (hex)",
			)
			.takes_value(true)
			.short("s")
			.required(false),
			// FIXME add merkle path, needed to compute nontrivial control blocks
		])
}

pub fn exec(
	matches: &clap::ArgMatches<'_>,
	client: &hal_simplicity::hal_simplicity_client::HalSimplicity,
) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory").to_string();
	let input_idx_str = matches.value_of("input-index").expect("input-idx is mandatory");
	let input_idx: u32 = input_idx_str.parse().expect("invalid input index");
	let input_utxo = matches.value_of("input-utxo").expect("input-utxo is mandatory").to_string();

	let internal_key = matches.value_of("internal-key").map(String::from);
	let cmr = matches.value_of("cmr").map(String::from);
	let state = matches.value_of("state").map(String::from);

	match client.pset_update_input(pset_b64, input_idx, input_utxo, internal_key, cmr, state) {
		Ok(response) => cmd::print_output(matches, &response),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
