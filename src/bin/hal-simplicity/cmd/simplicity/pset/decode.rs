// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use super::super::Error;
use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a PSET into JSON")
		.args(&cmd::opts_networks())
		.args(&[cmd::arg("pset", "PSET to decode (base64)").takes_value(true).required(true)])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("pset mandatory");
	match hal_simplicity::actions::simplicity::pset::pset_decode(pset_b64) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
