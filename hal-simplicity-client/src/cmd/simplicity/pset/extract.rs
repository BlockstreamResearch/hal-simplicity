// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;
use hal_simplicity::hal_simplicity_client::HalSimplicity;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("extract", "extract a raw transaction from a completed PSET")
		.args(&cmd::opts_networks())
		.args(&[cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true)])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>, client: &HalSimplicity) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory").to_string();

	match client.pset_extract(pset_b64) {
		Ok(response) => cmd::print_output(matches, &response),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
