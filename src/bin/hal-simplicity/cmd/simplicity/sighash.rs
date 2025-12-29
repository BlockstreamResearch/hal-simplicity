// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::Error;

pub fn cmd<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("sighash", "Compute signature hashes or signatures for use with Simplicity")
		.args(&cmd::opts_networks())
		.args(&[
			cmd::opt_yaml(),
			cmd::arg("tx", "transaction to sign (hex)").takes_value(true).required(true),
			cmd::arg("input-index", "the index of the input to sign (decimal)")
				.takes_value(true)
				.required(true),
			cmd::arg("cmr", "CMR of the input program (hex)").takes_value(true).required(true),
			cmd::arg("control-block", "Taproot control block of the input program (hex)")
				.takes_value(true)
				.required(false),
			cmd::opt("genesis-hash", "genesis hash of the blockchain the transaction belongs to (hex)")
				.short("g")
				.required(false),
			cmd::opt("secret-key", "secret key to sign the transaction with (hex)")
				.short("x")
				.takes_value(true)
				.required(false),
			cmd::opt("public-key", "public key which is checked against secret-key (if provided) and the signature (if provided) (hex)")
				.short("p")
				.takes_value(true)
				.required(false),
			cmd::opt("signature", "signature to validate (if provided, public-key must also be provided) (hex)")
				.short("s")
				.takes_value(true)
				.required(false),
			cmd::opt("input-utxo", "an input UTXO, without witnesses, in the form <scriptPubKey>:<asset ID or commitment>:<amount or value commitment> (should be used multiple times, one for each transaction input) (hex:hex:BTC decimal or hex)")
				.short("i")
				.multiple(true)
				.number_of_values(1)
				.required(false),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let tx_hex = matches.value_of("tx").expect("tx mandatory");
	let input_idx = matches.value_of("input-index").expect("input-idx is mandatory");
	let cmr = matches.value_of("cmr").expect("cmr is mandatory");
	let control_block = matches.value_of("control-block");
	let genesis_hash = matches.value_of("genesis-hash");
	let secret_key = matches.value_of("secret-key");
	let public_key = matches.value_of("public-key");
	let signature = matches.value_of("signature");
	let input_utxos: Option<Vec<_>> = matches.values_of("input-utxo").map(|vals| vals.collect());

	match hal_simplicity::actions::simplicity::simplicity_sighash(
		tx_hex,
		input_idx,
		cmr,
		control_block,
		genesis_hash,
		secret_key,
		public_key,
		signature,
		input_utxos.as_deref(),
	) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
	}
}
