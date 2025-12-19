use crate::simplicity::bitcoin::secp256k1::{
	schnorr, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey,
};
use crate::simplicity::elements;
use crate::simplicity::elements::hex::FromHex;
use crate::simplicity::elements::taproot::ControlBlock;
use crate::simplicity::jet::elements::{ElementsEnv, ElementsUtxo};
use crate::simplicity::Cmr;
use crate::types::{SimplicitySighashRequest, SimplicitySighashResponse};
use elements::bitcoin::secp256k1;
use elements::hashes::Hash as _;
use elements::pset::PartiallySignedTransaction;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimplicitySighashError {
	#[error("Error extracting transaction from PSET: {0}")]
	PsetExtraction(elements::pset::Error),

	#[error("Error parsing transaction hex: {0}")]
	TransactionHexParsing(elements::hex::Error),

	#[error("Error decoding transaction: {0}")]
	TransactionDecoding(elements::encode::Error),

	#[error("Error parsing CMR: {0}")]
	CmrParsing(elements::hashes::hex::HexToArrayError),

	#[error("Error parsing control block hex: {0}")]
	ControlBlockHexParsing(elements::hex::Error),

	#[error("Error decoding control block: {0}")]
	ControlBlockDecoding(elements::taproot::TaprootError),

	#[error("Input index {index} out-of-range for PSET with {n_inputs} inputs")]
	InputIndexOutOfRange {
		index: u32,
		n_inputs: usize,
	},

	#[error("Could not find control block in PSET for CMR {cmr}")]
	ControlBlockNotFound {
		cmr: String,
	},

	#[error("With a raw transaction, control-block must be provided")]
	ControlBlockRequired,

	#[error("Witness UTXO field not populated for input {input}")]
	WitnessUtxoMissing {
		input: usize,
	},

	#[error("With a raw transaction, input-utxos must be provided")]
	InputUtxosRequired,

	#[error("Expected {expected} input UTXOs but got {actual}")]
	InputUtxoCountMismatch {
		expected: usize,
		actual: usize,
	},

	#[error("Error parsing genesis hash: {0}")]
	GenesisHashParsing(elements::hashes::hex::HexToArrayError),

	#[error("Error parsing secret key: {0}")]
	SecretKeyParsing(secp256k1::Error),

	#[error("Secret key had public key {derived}, but was passed explicit public key {provided}")]
	PublicKeyMismatch {
		derived: String,
		provided: String,
	},

	#[error("Error parsing public key: {0}")]
	PublicKeyParsing(secp256k1::Error),

	#[error("Error parsing signature: {0}")]
	SignatureParsing(secp256k1::Error),

	#[error("If signature is provided, public-key must be provided as well")]
	SignatureWithoutPublicKey,

	#[error("Error parsing input UTXO: {0}")]
	InputUtxoParsing(String),

	#[error("Error parsing scriptPubKey hex: {0}")]
	ScriptPubKeyParsing(elements::hex::Error),

	#[error("Error parsing asset hex: {0}")]
	AssetHexParsing(elements::hashes::hex::HexToArrayError),

	#[error("Error parsing asset commitment hex: {0}")]
	AssetCommitmentHexParsing(elements::hex::Error),

	#[error("Error decoding asset commitment: {0}")]
	AssetCommitmentDecoding(elements::encode::Error),

	#[error("Error parsing value commitment hex: {0}")]
	ValueCommitmentHexParsing(elements::hex::Error),

	#[error("Error decoding value commitment: {0}")]
	ValueCommitmentDecoding(elements::encode::Error),
}

pub fn sighash(
	req: SimplicitySighashRequest,
) -> Result<SimplicitySighashResponse, SimplicitySighashError> {
	let secp = Secp256k1::new();

	// Attempt to decode transaction as PSET first
	let pset = req.tx.parse::<PartiallySignedTransaction>().ok();

	let tx = match pset {
		Some(ref pset) => pset.extract_tx().map_err(SimplicitySighashError::PsetExtraction)?,
		None => {
			let tx_bytes =
				Vec::from_hex(&req.tx).map_err(SimplicitySighashError::TransactionHexParsing)?;
			elements::encode::deserialize(&tx_bytes)
				.map_err(SimplicitySighashError::TransactionDecoding)?
		}
	};

	let cmr: Cmr = req.cmr.parse().map_err(SimplicitySighashError::CmrParsing)?;

	// If the user specifies a control block, use it. Otherwise query the PSET.
	let control_block = if let Some(cb) = req.control_block {
		let cb_bytes =
			Vec::from_hex(&cb).map_err(SimplicitySighashError::ControlBlockHexParsing)?;
		ControlBlock::from_slice(&cb_bytes).map_err(SimplicitySighashError::ControlBlockDecoding)?
	} else if let Some(ref pset) = pset {
		let n_inputs = pset.n_inputs();
		let input = pset.inputs().get(req.input_index as usize).ok_or_else(|| {
			SimplicitySighashError::InputIndexOutOfRange {
				index: req.input_index,
				n_inputs,
			}
		})?;

		let mut control_block = None;
		for (cb, script_ver) in &input.tap_scripts {
			if script_ver.1 == simplicity::leaf_version() && &script_ver.0[..] == cmr.as_ref() {
				control_block = Some(cb.clone());
			}
		}
		control_block.ok_or_else(|| SimplicitySighashError::ControlBlockNotFound {
			cmr: cmr.to_string(),
		})?
	} else {
		return Err(SimplicitySighashError::ControlBlockRequired);
	};

	let input_utxos = if let Some(input_utxos) = req.input_utxos {
		input_utxos
			.iter()
			.map(|utxo_str| parse_elements_utxo(utxo_str))
			.collect::<Result<Vec<_>, _>>()?
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
			.collect::<Result<Vec<_>, _>>()?
	} else {
		return Err(SimplicitySighashError::InputUtxosRequired);
	};

	if input_utxos.len() != tx.input.len() {
		return Err(SimplicitySighashError::InputUtxoCountMismatch {
			expected: tx.input.len(),
			actual: input_utxos.len(),
		});
	}

	// Default to Bitcoin blockhash
	let genesis_hash = match req.genesis_hash {
		Some(s) => s.parse().map_err(SimplicitySighashError::GenesisHashParsing)?,
		None => elements::BlockHash::from_byte_array([
			0xc1, 0xb1, 0x6a, 0xe2, 0x4f, 0x24, 0x23, 0xae, 0xa2, 0xea, 0x34, 0x55, 0x22, 0x92,
			0x79, 0x3b, 0x5b, 0x5e, 0x82, 0x99, 0x9a, 0x1e, 0xed, 0x81, 0xd5, 0x6a, 0xee, 0x52,
			0x8e, 0xda, 0x71, 0xa7,
		]),
	};

	let tx_env =
		ElementsEnv::new(&tx, input_utxos, req.input_index, cmr, control_block, None, genesis_hash);

	let (pk, sig) = match (req.public_key, req.signature) {
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
	let sighash_msg = Message::from_digest(sighash.to_byte_array());

	Ok(SimplicitySighashResponse {
		sighash,
		signature: match req.secret_key {
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

fn parse_elements_utxo(s: &str) -> Result<ElementsUtxo, SimplicitySighashError> {
	use crate::simplicity::bitcoin::{Amount, Denomination};

	let parts: Vec<&str> = s.split(':').collect();
	if parts.len() != 3 {
		return Err(SimplicitySighashError::InputUtxoParsing(
			"expected format <scriptPubKey>:<asset>:<value>".to_string(),
		));
	}

	let script_pubkey: elements::Script =
		parts[0].parse().map_err(SimplicitySighashError::ScriptPubKeyParsing)?;

	let asset = if parts[1].len() == 64 {
		let asset_id: elements::AssetId =
			parts[1].parse().map_err(SimplicitySighashError::AssetHexParsing)?;
		elements::confidential::Asset::Explicit(asset_id)
	} else {
		let commitment_bytes =
			Vec::from_hex(parts[1]).map_err(SimplicitySighashError::AssetCommitmentHexParsing)?;
		elements::confidential::Asset::from_commitment(&commitment_bytes)
			.map_err(SimplicitySighashError::AssetCommitmentDecoding)?
	};

	let value = if let Ok(btc_amount) = Amount::from_str_in(parts[2], Denomination::Bitcoin) {
		elements::confidential::Value::Explicit(btc_amount.to_sat())
	} else {
		let commitment_bytes =
			Vec::from_hex(parts[2]).map_err(SimplicitySighashError::ValueCommitmentHexParsing)?;
		elements::confidential::Value::from_commitment(&commitment_bytes)
			.map_err(SimplicitySighashError::ValueCommitmentDecoding)?
	};

	Ok(ElementsUtxo {
		script_pubkey,
		asset,
		value,
	})
}
