// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use elements::encode::serialize_hex;

use super::super::Error;
use crate::cmd::{self, simplicity::pset::PsetError};

#[derive(Debug, thiserror::Error)]
pub enum PsetExtractError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("invalid PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("ailed to extract transaction: {0}")]
	TransactionExtract(elements::pset::Error),
}

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("extract", "extract a raw transaction from a completed PSET")
		.args(&cmd::opts_networks())
		.args(&[cmd::arg("pset", "PSET to update (base64)").takes_value(true).required(true)])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let pset_b64 = matches.value_of("pset").expect("tx mandatory");
	match exec_inner(pset_b64) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}

fn exec_inner(pset_b64: &str) -> Result<String, PsetExtractError> {
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetExtractError::PsetDecode)?;

	let tx = pset.extract_tx().map_err(PsetExtractError::TransactionExtract)?;
	Ok(serialize_hex(&tx))
}
