// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::{Error, ErrorExt as _};

use elements::hashes::Hash;
use hal_simplicity::simplicity::bitcoin::secp256k1::{
	schnorr, Keypair, Message, Secp256k1, SecretKey,
};
use hal_simplicity::simplicity::bitcoin::{Amount, Denomination};
use hal_simplicity::simplicity::elements::hashes::sha256;
use hal_simplicity::simplicity::elements::hex::FromHex;
use hal_simplicity::simplicity::elements::taproot::ControlBlock;
use hal_simplicity::simplicity::elements::{self, confidential, Transaction};
use hal_simplicity::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use hal_simplicity::simplicity::Cmr;

use serde::Serialize;

#[derive(Serialize)]
struct SighashInfo {
	sighash: sha256::Hash,
	signature: Option<schnorr::Signature>,
	valid_signature: Option<bool>,
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
	let public_key = matches.value_of("public-key");
	let signature = matches.value_of("signature");
	let input_utxos: Vec<_> = matches.values_of("input-utxo").unwrap().collect();

	match exec_inner(
		tx_hex,
		input_idx,
		cmr,
		control_block,
		genesis_hash,
		secret_key,
		public_key,
		signature,
		&input_utxos,
	) {
		Ok(info) => cmd::print_output(matches, &info),
		Err(e) => cmd::print_output(matches, &e),
	}
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
	tx_hex: &str,
	input_idx: &str,
	cmr: &str,
	control_block: &str,
	genesis_hash: Option<&str>,
	secret_key: Option<&str>,
	public_key: Option<&str>,
	signature: Option<&str>,
	input_utxos: &[&str],
) -> Result<SighashInfo, Error> {
	let secp = Secp256k1::new();

	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let tx_bytes = Vec::from_hex(tx_hex).result_context("parsing transaction hex")?;
	let tx: Transaction =
		elements::encode::deserialize(&tx_bytes).result_context("decoding transaction")?;
	let input_idx: u32 = input_idx.parse().result_context("parsing input-idx")?;
	let cmr: Cmr = cmr.parse().result_context("parsing cmr")?;

	let cb_bytes = Vec::from_hex(control_block).result_context("parsing control block hex")?;
	// For txes from webide, the internal key in this control block will be the hardcoded
	// value f5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2
	let control_block =
		ControlBlock::from_slice(&cb_bytes).result_context("decoding control block")?;

	let input_utxos = input_utxos
		.iter()
		.map(|utxo_str| parse_elements_utxo(utxo_str))
		.collect::<Result<Vec<_>, Error>>()?;
	assert_eq!(input_utxos.len(), tx.input.len());

	// Default to Bitcoin blockhash.
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().result_context("parsing genesis hash")?,
		None => elements::BlockHash::from_byte_array([
			// copied out of simplicity-webide source
			0xc1, 0xb1, 0x6a, 0xe2, 0x4f, 0x24, 0x23, 0xae, 0xa2, 0xea, 0x34, 0x55, 0x22, 0x92,
			0x79, 0x3b, 0x5b, 0x5e, 0x82, 0x99, 0x9a, 0x1e, 0xed, 0x81, 0xd5, 0x6a, 0xee, 0x52,
			0x8e, 0xda, 0x71, 0xa7,
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

	let (pk, sig) = match (public_key, signature) {
		(Some(pk), None) => (Some(pk.parse().result_context("parsing public key")?), None),
		(Some(pk), Some(sig)) => (
			Some(pk.parse().result_context("parsing public key")?),
			Some(sig.parse().result_context("parsing signature")?),
		),
		(None, Some(_)) => {
			return Err(Error {
				context: "reading cli arguments",
				error: "if signature is provided, public-key must be provided as well".to_owned(),
			})
		}
		(None, None) => (None, None),
	};

	let sighash = tx_env.c_tx_env().sighash_all();
	let sighash_msg = Message::from_digest(sighash.to_byte_array()); // FIXME can remove in next version ofrust-secp
	Ok(SighashInfo {
		sighash,
		signature: match secret_key {
			Some(sk) => {
				let sk: SecretKey = sk.parse().result_context("parsing secret key hex")?;
				let keypair = Keypair::from_secret_key(&secp, &sk);

				if let Some(ref pk) = pk {
					if pk != &keypair.x_only_public_key().0 {
						return Err(Error {
							context: "checking secret key and public key consistency",
							error: format!(
								"secret key had public key {}, but was passed explicit public key {}",
								keypair.x_only_public_key().0,
								pk,
							),
						});
					}
				}

				Some(secp.sign_schnorr(&sighash_msg, &keypair))
			}
			None => None,
		},
		valid_signature: match (pk, sig) {
			(Some(pk), Some(sig)) => Some(secp.verify_schnorr(&sig, &sighash_msg, &pk).is_ok()),
			_ => None,
		},
	})
}
