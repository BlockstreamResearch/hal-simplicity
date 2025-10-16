// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::{Error, ErrorExt as _};

use elements::hashes::Hash;
use hal_simplicity::simplicity::bitcoin::secp256k1::{
	schnorr, Keypair, Message, Secp256k1, SecretKey,
};
use hal_simplicity::simplicity::elements::hashes::sha256;
use hal_simplicity::simplicity::elements::hex::FromHex;
use hal_simplicity::simplicity::elements::taproot::ControlBlock;
use hal_simplicity::simplicity::elements::{self, Transaction};
use hal_simplicity::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use hal_simplicity::simplicity::Cmr;

use serde::Serialize;

#[derive(Serialize)]
struct SighashInfo {
	sighash: sha256::Hash,
	signature: Option<schnorr::Signature>,
}

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
			cmd::arg("control-block", "Taproot control block of the input program (hex)").takes_value(true).required(true),
			cmd::opt("genesis-hash", "genesis hash of the blockchain the transaction belongs to (hex)")
				.short("g")
				.required(false),
			cmd::opt("secret-key", "secret key to sign the transaction with (hex)")
				.short("s")
				.required(false),
			cmd::opt("input-utxo", "an input UTXO, without witnesses (should be used multiple times, one for each transaction input) (hex)")
				.short("i")
				.multiple(true)
				.number_of_values(1)
				.required(true),
		])
}

pub fn exec<'a>(matches: &clap::ArgMatches<'a>) {
	let tx_hex = matches.value_of("tx").expect("tx mandatory");
	let input_idx = matches.value_of("input-index").expect("input-idx is mandatory");
	let cmr = matches.value_of("cmr").expect("cmr is mandatory");
	let control_block = matches.value_of("control-block").expect("control-block is mandatory");
	let genesis_hash = matches.value_of("genesis-hash");
	let secret_key = matches.value_of("secret-key");
	let input_utxos: Vec<_> = matches.values_of("input-utxo").unwrap().collect();

	match exec_inner(tx_hex, input_idx, cmr, control_block, genesis_hash, secret_key, &input_utxos)
	{
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(matches, &e),
	}
}

fn exec_inner(
	tx_hex: &str,
	input_idx: &str,
	cmr: &str,
	control_block: &str,
	genesis_hash: Option<&str>,
	secret_key: Option<&str>,
	input_utxos: &[&str],
) -> Result<SighashInfo, Error> {
	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let tx_bytes = Vec::from_hex(tx_hex).result_context("parsing transaction hex")?;
	let tx: Transaction =
		elements::encode::deserialize(&tx_bytes).result_context("decoding transaction")?;
	let input_idx: u32 = input_idx.parse().result_context("parsing input-idx")?;
	let cmr: Cmr = cmr.parse().result_context("parsing cmr")?;

	let cb_bytes = Vec::from_hex(control_block).result_context("parsing control block hex")?;
	let control_block =
		ControlBlock::from_slice(&cb_bytes).result_context("decoding control block")?;

	let input_utxos = input_utxos
		.iter()
		.map(|utxo_hex| {
			let utxo_bytes = Vec::from_hex(utxo_hex).result_context("parsing input UTXO hex")?;
			let utxo: elements::TxOut =
				elements::encode::deserialize(&utxo_bytes).result_context("decoding input UTXO")?;
			Ok(ElementsUtxo::from(utxo))
		})
		.collect::<Result<Vec<_>, Error>>()?;
	assert_eq!(input_utxos.len(), tx.input.len());

	// Default to Bitcoin blockhash.
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().result_context("parsing genesis hash")?,
		None => elements::BlockHash::from_byte_array([
			0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0xd6, 0x68, 0x9c, 0x08, 0x5a, 0xe1, 0x65, 0x83,
			0x1e, 0x93, 0x4f, 0xf7, 0x63, 0xae, 0x46, 0xa2, 0xa6, 0xc1, 0x72, 0xb3, 0xf1, 0xb6,
			0x0a, 0x8c, 0xe2, 0x6f,
		]),
	};

	let tx_env = ElementsEnv::new(
		&tx,
		input_utxos,
		input_idx,
		cmr,
		control_block,
		None, // FIXME populate this; needs https://github.com/BlockstreamResearch/rust-simplicity/issues/315 first
		genesis_hash,
	);

	let sighash = tx_env.c_tx_env().sighash_all();
	let sighash_msg = Message::from_digest(sighash.to_byte_array()); // FIXME can remove in next version ofrust-secp
	Ok(SighashInfo {
		sighash,
		signature: match secret_key {
			Some(sk) => {
				let secp = Secp256k1::new();
				let sk: SecretKey = sk.parse().result_context("parsing secret key hex")?;
				let keypair = Keypair::from_secret_key(&secp, &sk);
				Some(secp.sign_schnorr(&sighash_msg, &keypair))
			}
			None => None,
		},
	})
}
