// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;

use super::{Error, ErrorExt as _};

use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use hal_simplicity_daemon::simplicity::bitcoin::secp256k1::{
	schnorr, Keypair, Message, Secp256k1, SecretKey,
};
use hal_simplicity_daemon::simplicity::elements;
use hal_simplicity_daemon::simplicity::elements::hashes::sha256;
use hal_simplicity_daemon::simplicity::elements::hex::FromHex;
use hal_simplicity_daemon::simplicity::elements::taproot::ControlBlock;

use hal_simplicity_daemon::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use hal_simplicity_daemon::simplicity::Cmr;

use serde::Serialize;

#[derive(Serialize)]
struct SighashInfo {
	sighash: sha256::Hash,
	signature: Option<schnorr::Signature>,
	valid_signature: Option<bool>,
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

	match exec_inner(
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
		Err(e) => cmd::print_output(matches, &e),
	}
}

#[allow(clippy::too_many_arguments)]
fn exec_inner(
	tx_hex: &str,
	input_idx: &str,
	cmr: &str,
	control_block: Option<&str>,
	genesis_hash: Option<&str>,
	secret_key: Option<&str>,
	public_key: Option<&str>,
	signature: Option<&str>,
	input_utxos: Option<&[&str]>,
) -> Result<SighashInfo, Error> {
	let secp = Secp256k1::new();

	// Attempt to decode transaction as PSET first. If it succeeds, we can extract
	// a lot of information from it. If not, we assume the transaction is hex and
	// will give the user an error corresponding to this.
	let pset = tx_hex.parse::<PartiallySignedTransaction>().ok();

	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let tx = match pset {
		Some(ref pset) => pset.extract_tx().result_context("extracting transaction from PSET")?,
		None => {
			let tx_bytes = Vec::from_hex(tx_hex).result_context("parsing transaction hex")?;
			elements::encode::deserialize(&tx_bytes).result_context("decoding transaction")?
		}
	};
	let input_idx: u32 = input_idx.parse().result_context("parsing input-idx")?;
	let cmr: Cmr = cmr.parse().result_context("parsing cmr")?;

	// If the user specifies a control block, use it. Otherwise query the PSET.
	let control_block = if let Some(cb) = control_block {
		let cb_bytes = Vec::from_hex(cb).result_context("parsing control block hex")?;
		// For txes from webide, the internal key in this control block will be the hardcoded
		// value f5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2
		ControlBlock::from_slice(&cb_bytes).result_context("decoding control block")?
	} else if let Some(ref pset) = pset {
		let n_inputs = pset.n_inputs();
		let input = pset
			.inputs()
			.get(input_idx as usize) // cast u32->usize probably fine
			.ok_or_else(|| {
				format!("index {} out-of-range for PSET with {} inputs", input_idx, n_inputs)
			})
			.result_context("parsing input index")?;

		let mut control_block = None;
		for (cb, script_ver) in &input.tap_scripts {
			if script_ver.1 == simplicity::leaf_version() && &script_ver.0[..] == cmr.as_ref() {
				control_block = Some(cb.clone());
			}
		}
		match control_block {
			Some(cb) => cb,
			None => {
				return Err(format!("could not find control block in PSET for CMR {}", cmr))
					.result_context("finding control block")?
			}
		}
	} else {
		return Err("with a raw transaction, control-block must be provided")
			.result_context("computing control block");
	};

	let input_utxos = if let Some(input_utxos) = input_utxos {
		input_utxos
			.iter()
			.map(|utxo_str| super::parse_elements_utxo(utxo_str))
			.collect::<Result<Vec<_>, Error>>()?
	} else if let Some(ref pset) = pset {
		pset.inputs()
			.iter()
			.enumerate()
			.map(|(n, input)| match input.witness_utxo {
				Some(ref utxo) => Ok(ElementsUtxo {
					script_pubkey: utxo.script_pubkey.clone(),
					asset: utxo.asset,
					value: utxo.value,
				}),
				None => Err(format!("witness_utxo field not populated for input {n}")),
			})
			.collect::<Result<Vec<_>, _>>()
			.result_context("extracting input UTXO information")?
	} else {
		return Err("with a raw transaction, input-utxos must be provided")
			.result_context("computing control block");
	};
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
