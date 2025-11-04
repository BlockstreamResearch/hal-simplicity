// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use elements::encode::serialize_hex;

use super::super::{Error, ErrorExt as _};
use crate::cmd;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("extract", "extract a raw transaction from a completed PSET")
		.args(&cmd::opts_networks())
		.args(&[cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true)])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory");
	match exec_inner(pset_b64) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(matches, &e),
	}
}

fn exec_inner(pset_b64: &str) -> Result<String, Error> {
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().result_context("decoding PSET")?;

	let tx = pset.extract_tx().result_context("extracting transaction")?;
	Ok(serialize_hex(&tx))
}
