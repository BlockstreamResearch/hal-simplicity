// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

mod info;
mod pset;
mod sighash;

use crate::cmd;
use hal_simplicity_daemon::simplicity::bitcoin::{Amount, Denomination};
use hal_simplicity_daemon::simplicity::elements::confidential;
use hal_simplicity_daemon::simplicity::elements::hex::FromHex as _;
use hal_simplicity_daemon::simplicity::jet::elements::ElementsUtxo;

use serde::Serialize;

#[derive(Serialize)]
struct Error {
	context: &'static str,
	error: String,
}

trait ErrorExt<T> {
	fn result_context(self, context: &'static str) -> Result<T, Error>;
}

impl<T, E: core::fmt::Display> ErrorExt<T> for Result<T, E> {
	fn result_context(self, context: &'static str) -> Result<T, Error> {
		self.map_err(|e| Error {
			context,
			error: e.to_string(),
		})
	}
}

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("simplicity", "manipulate Simplicity programs")
		.subcommand(self::info::cmd())
		.subcommand(self::pset::cmd())
		.subcommand(self::sighash::cmd())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("info", Some(m)) => self::info::exec(m),
		("pset", Some(m)) => self::pset::exec(m),
		("sighash", Some(m)) => self::sighash::exec(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn parse_elements_utxo(s: &str) -> Result<ElementsUtxo, Error> {
	let parts: Vec<&str> = s.split(':').collect();
	if parts.len() != 3 {
		return Err(Error {
			context: "parsing input UTXO",
			error: "expected format <scriptPubKey>:<asset>:<value>".to_string(),
		});
	}
	// Parse scriptPubKey
	let script_pubkey: elements::Script =
		parts[0].parse().result_context("parsing scriptPubKey hex")?;

	// Parse asset - try as explicit AssetId first, then as confidential commitment
	let asset = if parts[1].len() == 64 {
		// 32 bytes = explicit AssetId
		let asset_id: elements::AssetId = parts[1].parse().result_context("parsing asset hex")?;
		confidential::Asset::Explicit(asset_id)
	} else {
		// Parse anything except 32 bytes as a confidential commitment (which must be 33 bytes)
		let commitment_bytes =
			Vec::from_hex(parts[1]).result_context("parsing asset commitment hex")?;
		elements::confidential::Asset::from_commitment(&commitment_bytes)
			.result_context("decoding asset commitment")?
	};

	// Parse value - try as BTC decimal first, then as confidential commitment
	let value = if let Ok(btc_amount) = Amount::from_str_in(parts[2], Denomination::Bitcoin) {
		// Explicit value in BTC
		elements::confidential::Value::Explicit(btc_amount.to_sat())
	} else {
		// 33 bytes = confidential commitment
		let commitment_bytes =
			Vec::from_hex(parts[2]).result_context("parsing value commitment hex")?;
		elements::confidential::Value::from_commitment(&commitment_bytes)
			.result_context("decoding value commitment")?
	};

	Ok(ElementsUtxo {
		script_pubkey,
		asset,
		value,
	})
}
