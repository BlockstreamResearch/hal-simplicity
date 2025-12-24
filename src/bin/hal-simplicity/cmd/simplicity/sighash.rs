// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use crate::cmd;
use crate::cmd::simplicity::ParseElementsUtxoError;

use super::Error;

use elements::bitcoin::secp256k1;
use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use hal_simplicity::simplicity::bitcoin::secp256k1::{
	schnorr, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey,
};
use hal_simplicity::simplicity::elements;
use hal_simplicity::simplicity::elements::hashes::sha256;
use hal_simplicity::simplicity::elements::hex::FromHex;
use hal_simplicity::simplicity::elements::taproot::ControlBlock;

use hal_simplicity::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use hal_simplicity::simplicity::Cmr;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum SimplicitySighashError {
	#[error("failed extracting transaction from PSET: {0}")]
	PsetExtraction(elements::pset::Error),

	#[error("invalid transaction hex: {0}")]
	TransactionHexParsing(elements::hex::Error),

	#[error("invalid transaction decoding: {0}")]
	TransactionDecoding(elements::encode::Error),

	#[error("invalid input index: {0}")]
	InputIndexParsing(std::num::ParseIntError),

	#[error("invalid CMR: {0}")]
	CmrParsing(elements::hashes::hex::HexToArrayError),

	#[error("invalid control block hex: {0}")]
	ControlBlockHexParsing(elements::hex::Error),

	#[error("invalid control block decoding: {0}")]
	ControlBlockDecoding(elements::taproot::TaprootError),

	#[error("input index {index} out-of-range for PSET with {n_inputs} inputs")]
	InputIndexOutOfRange {
		index: u32,
		n_inputs: usize,
	},

	#[error("could not find control block in PSET for CMR {cmr}")]
	ControlBlockNotFound {
		cmr: String,
	},

	#[error("with a raw transaction, control-block must be provided")]
	ControlBlockRequired,

	#[error("witness UTXO field not populated for input {input}")]
	WitnessUtxoMissing {
		input: usize,
	},

	#[error("with a raw transaction, input-utxos must be provided")]
	InputUtxosRequired,

	#[error("expected {expected} input UTXOs but got {actual}")]
	InputUtxoCountMismatch {
		expected: usize,
		actual: usize,
	},

	#[error("invalid genesis hash: {0}")]
	GenesisHashParsing(elements::hashes::hex::HexToArrayError),

	#[error("invalid secret key: {0}")]
	SecretKeyParsing(secp256k1::Error),

	#[error("secret key had public key {derived}, but was passed explicit public key {provided}")]
	PublicKeyMismatch {
		derived: String,
		provided: String,
	},

	#[error("invalid public key: {0}")]
	PublicKeyParsing(secp256k1::Error),

	#[error("invalid signature: {0}")]
	SignatureParsing(secp256k1::Error),

	#[error("if signature is provided, public-key must be provided as well")]
	SignatureWithoutPublicKey,

	#[error("invalid input UTXO: {0}")]
	InputUtxoParsing(ParseElementsUtxoError),
}

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
		Err(e) => cmd::print_output(
			matches,
			&Error {
				error: format!("{}", e),
			},
		),
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
) -> Result<SighashInfo, SimplicitySighashError> {
	let secp = Secp256k1::new();

	// Attempt to decode transaction as PSET first. If it succeeds, we can extract
	// a lot of information from it. If not, we assume the transaction is hex and
	// will give the user an error corresponding to this.
	let pset = tx_hex.parse::<PartiallySignedTransaction>().ok();

	// In the future we should attempt to parse as a Bitcoin program if parsing as
	// Elements fails. May be tricky/annoying in Rust since Program<Elements> is a
	// different type from Program<Bitcoin>.
	let tx = match pset {
		Some(ref pset) => pset.extract_tx().map_err(SimplicitySighashError::PsetExtraction)?,
		None => {
			let tx_bytes =
				Vec::from_hex(tx_hex).map_err(SimplicitySighashError::TransactionHexParsing)?;
			elements::encode::deserialize(&tx_bytes)
				.map_err(SimplicitySighashError::TransactionDecoding)?
		}
	};
	let input_idx: u32 = input_idx.parse().map_err(SimplicitySighashError::InputIndexParsing)?;
	let cmr: Cmr = cmr.parse().map_err(SimplicitySighashError::CmrParsing)?;

	// If the user specifies a control block, use it. Otherwise query the PSET.
	let control_block = if let Some(cb) = control_block {
		let cb_bytes = Vec::from_hex(cb).map_err(SimplicitySighashError::ControlBlockHexParsing)?;
		// For txes from webide, the internal key in this control block will be the hardcoded
		// value f5919fa64ce45f8306849072b26c1bfdd2937e6b81774796ff372bd1eb5362d2
		ControlBlock::from_slice(&cb_bytes).map_err(SimplicitySighashError::ControlBlockDecoding)?
	} else if let Some(ref pset) = pset {
		let n_inputs = pset.n_inputs();
		let input = pset
			.inputs()
			.get(input_idx as usize) // cast u32->usize probably fine
			.ok_or(SimplicitySighashError::InputIndexOutOfRange {
				index: input_idx,
				n_inputs,
			})?;

		let mut control_block = None;
		for (cb, script_ver) in &input.tap_scripts {
			if script_ver.1 == simplicity::leaf_version() && &script_ver.0[..] == cmr.as_ref() {
				control_block = Some(cb.clone());
			}
		}
		match control_block {
			Some(cb) => cb,
			None => {
				return Err(SimplicitySighashError::ControlBlockNotFound {
					cmr: cmr.to_string(),
				})
			}
		}
	} else {
		return Err(SimplicitySighashError::ControlBlockRequired);
	};

	let input_utxos = if let Some(input_utxos) = input_utxos {
		input_utxos
			.iter()
			.map(|utxo_str| {
				super::parse_elements_utxo(utxo_str)
					.map_err(SimplicitySighashError::InputUtxoParsing)
			})
			.collect::<Result<Vec<_>, SimplicitySighashError>>()?
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
				None => Err(SimplicitySighashError::WitnessUtxoMissing {
					input: n,
				}),
			})
			.collect::<Result<Vec<_>, SimplicitySighashError>>()?
	} else {
		return Err(SimplicitySighashError::InputUtxosRequired);
	};
	if input_utxos.len() != tx.input.len() {
		return Err(SimplicitySighashError::InputUtxoCountMismatch {
			expected: tx.input.len(),
			actual: input_utxos.len(),
		});
	}

	// Default to Bitcoin blockhash.
	let genesis_hash = match genesis_hash {
		Some(s) => s.parse().map_err(SimplicitySighashError::GenesisHashParsing)?,
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
		(Some(pk), None) => (
			Some(pk.parse::<XOnlyPublicKey>().map_err(SimplicitySighashError::PublicKeyParsing)?),
			None,
		),
		(Some(pk), Some(sig)) => (
			Some(pk.parse::<XOnlyPublicKey>().map_err(SimplicitySighashError::PublicKeyParsing)?),
			Some(
				sig.parse::<schnorr::Signature>()
					.map_err(SimplicitySighashError::SignatureParsing)?,
			),
		),
		(None, Some(_)) => return Err(SimplicitySighashError::SignatureWithoutPublicKey),
		(None, None) => (None, None),
	};

	let sighash = tx_env.c_tx_env().sighash_all();
	let sighash_msg = Message::from_digest(sighash.to_byte_array()); // FIXME can remove in next version ofrust-secp
	Ok(SighashInfo {
		sighash,
		signature: match secret_key {
			Some(sk) => {
				let sk: SecretKey = sk.parse().map_err(SimplicitySighashError::SecretKeyParsing)?;
				let keypair = Keypair::from_secret_key(&secp, &sk);

				if let Some(ref pk) = pk {
					if pk != &keypair.x_only_public_key().0 {
						return Err(SimplicitySighashError::PublicKeyMismatch {
							derived: keypair.x_only_public_key().0.to_string(),
							provided: pk.to_string(),
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
