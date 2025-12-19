// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

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

pub fn exec<'a>(
	matches: &clap::ArgMatches<'a>,
	client: &hal_simplicity::hal_simplicity_client::HalSimplicity,
) {
	let inputs_json = matches.value_of("inputs").expect("inputs mandatory").to_string();
	let outputs_json = matches.value_of("outputs").expect("outputs mandatory").to_string();
	let network = if matches.is_present("liquid") {
		Some("liquid".to_string())
	} else {
		Some("elementsregtest".to_string())
	};

	match client.pset_create(inputs_json, outputs_json, network) {
		Ok(response) => cmd::print_output(matches, &response),
		Err(e) => {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}
}
