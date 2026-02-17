// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use clap::value_t;
use hal_simplicity::actions::simplicity::{GraphFormat, SharingLevel};

use crate::cmd;

use super::Error;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("graph", "Parse a base64-encoded Simplicity program and display a graph").args(
		&[
			cmd::arg("program", "a Simplicity program in base64").takes_value(true).required(true),
			cmd::arg("witness", "a hex encoding of all the witness data for the program")
				.takes_value(true)
				.required(false),
			cmd::arg(
				"sharing",
				"the level of node sharing to use when displaying. Either none or max",
			)
			.short("s")
			.long("sharing")
			.takes_value(true)
			.required(false)
			.default_value("none")
			.possible_values(&["none", "max"]),
			cmd::arg("format", "the format for the graph, either graphviz (alias dot) or mermaid")
				.long("format")
				.takes_value(true)
				.required(false)
				.default_value("graphviz")
				.possible_values(&["graphviz", "dot", "mermaid"]),
		],
	)
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let program = matches.value_of("program").expect("program is mandatory");
	let witness = matches.value_of("witness");
	let sharing = value_t!(matches, "sharing", SharingLevel).unwrap_or(SharingLevel::NoSharing);
	let format = value_t!(matches, "format", GraphFormat).unwrap_or(GraphFormat::Dot);

	match hal_simplicity::actions::simplicity::simplicity_graph(program, witness, sharing, format) {
		Ok(graph) => println!("{}", graph),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
