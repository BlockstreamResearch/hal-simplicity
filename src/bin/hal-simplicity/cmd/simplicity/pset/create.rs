// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use super::super::Error;
use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create an empty PSET").args(&cmd::opts_networks()).args(&[
		cmd::arg(
			"inputs",
			"input outpoints (JSON array of objects containing txid, vout, sequence)",
		)
		.takes_value(true)
		.required(true),
		cmd::arg("outputs", "outputs (JSON array of objects containing address, asset, amount)")
			.takes_value(true)
			.required(true),
	])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let inputs_json = matches.value_of("inputs").expect("inputs mandatory");
	let outputs_json = matches.value_of("outputs").expect("inputs mandatory");

	match hal_simplicity::actions::simplicity::pset::pset_create(inputs_json, outputs_json) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
