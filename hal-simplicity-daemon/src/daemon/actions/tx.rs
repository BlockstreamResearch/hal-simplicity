use std::convert::TryInto;

use elements::bitcoin::{self, secp256k1};
use elements::encode::{deserialize, serialize};
use elements::hashes::Hash;
use elements::secp256k1_zkp::{
	Generator, PedersenCommitment, PublicKey, RangeProof, SurjectionProof, Tweak,
};
use elements::{
	confidential, AssetIssuance, OutPoint, Script, Transaction, TxIn, TxInWitness, TxOut,
	TxOutWitness,
};
use hex::FromHexError;
use thiserror::Error;

use crate::utils::confidential::{
	ConfidentialAssetInfo, ConfidentialNonceInfo, ConfidentialType, ConfidentialValueInfo,
};
use crate::utils::tx::{
	AssetIssuanceInfo, InputInfo, InputScriptInfo, InputWitnessInfo, OutputInfo, OutputScriptInfo,
	OutputWitnessInfo, PeginDataInfo, PegoutDataInfo, TransactionInfo,
};

use crate::types::Network;
use crate::utils::GetInfo;

use crate::types::{TxCreateRequest, TxDecodeRequest};

#[derive(Debug, Error)]
pub enum TxError {
	#[error("Failed to parse transaction info JSON: {0}")]
	JsonParse(serde_json::Error),

	#[error("Failed to decode raw transaction hex: {0}")]
	TxHex(FromHexError),

	#[error("Invalid transaction format: {0}")]
	TxDeserialize(elements::encode::Error),

	#[error("Failed to serialize response: {0}")]
	Serialize(serde_json::Error),

	#[error("Failed to parse network: {0}")]
	NetworkParse(String),

	#[error("{field} is required")]
	MissingField {
		field: String,
	},

	#[error("Invalid prevout format: {0}")]
	PrevoutParse(bitcoin::blockdata::transaction::ParseOutPointError),

	#[error("txid field given without vout field")]
	MissingVout,

	#[error("Conflicting prevout information")]
	ConflictingPrevout,

	#[error("No previous output provided")]
	NoPrevout,

	#[error("Invalid confidential commitment: {0}")]
	ConfidentialCommitment(elements::secp256k1_zkp::Error),

	#[error("Invalid confidential publicKey: {0}")]
	ConfidentialCommitmentPublicKey(secp256k1::Error),

	#[error("Wrong size of nonce field")]
	NonceSize,

	#[error("Invalid size of asset_entropy")]
	AssetEntropySize,

	#[error("Invalid asset_blinding_nonce: {0}")]
	AssetBlindingNonce(elements::secp256k1_zkp::Error),

	#[error("Decoding script assembly is not yet supported")]
	AsmNotSupported,

	#[error("No scriptSig info provided")]
	NoScriptSig,

	#[error("No scriptPubKey info provided")]
	NoScriptPubKey,

	#[error("Invalid outpoint in pegin_data: {0}")]
	PeginOutpoint(bitcoin::blockdata::transaction::ParseOutPointError),

	#[error("Outpoint in pegin_data does not correspond to input value")]
	PeginOutpointMismatch,

	#[error("Asset in pegin_data should be explicit")]
	PeginAssetNotExplicit,

	#[error("Invalid rangeproof: {0}")]
	RangeProof(elements::secp256k1_zkp::Error),

	#[error("Invalid sequence: {0}")]
	Sequence(core::num::TryFromIntError),

	#[error("Addresses for different networks are used in the output scripts")]
	MixedNetworks,

	#[error("Invalid surjection proof: {0}")]
	SurjectionProof(elements::secp256k1_zkp::Error),

	#[error("Value in pegout_data does not correspond to output value")]
	PegoutValueMismatch,

	#[error("Explicit value is required for pegout data")]
	PegoutValueNotExplicit,

	#[error("Asset in pegout_data does not correspond to output value")]
	PegoutAssetMismatch,
}

pub fn create(req: TxCreateRequest) -> Result<String, TxError> {
	let info = serde_json::from_str::<TransactionInfo>(&req.tx_info).map_err(TxError::JsonParse)?;

	let tx = create_transaction(info)?;
	let tx_bytes = serialize(&tx);
	Ok(hex::encode(&tx_bytes))
}

pub fn decode(req: TxDecodeRequest) -> Result<serde_json::Value, TxError> {
	let raw_tx = hex::decode(&req.raw_tx).map_err(TxError::TxHex)?;

	let tx: Transaction = deserialize(&raw_tx).map_err(TxError::TxDeserialize)?;

	let network =
		req.network.as_deref().map(parse_network).transpose()?.unwrap_or(Network::ElementsRegtest);

	let info = GetInfo::get_info(&tx, network);
	serde_json::to_value(&info).map_err(TxError::Serialize)
}

pub fn create_transaction(info: TransactionInfo) -> Result<Transaction, TxError> {
	Ok(Transaction {
		version: info.version.ok_or(TxError::MissingField {
			field: "version".to_string(),
		})?,
		lock_time: info.locktime.ok_or(TxError::MissingField {
			field: "locktime".to_string(),
		})?,
		input: info
			.inputs
			.ok_or(TxError::MissingField {
				field: "inputs".to_string(),
			})?
			.into_iter()
			.map(create_input)
			.collect::<Result<Vec<_>, _>>()?,
		output: info
			.outputs
			.ok_or(TxError::MissingField {
				field: "outputs".to_string(),
			})?
			.into_iter()
			.map(create_output)
			.collect::<Result<Vec<_>, _>>()?,
	})
}

fn outpoint_from_input_info(input: &InputInfo) -> Result<OutPoint, TxError> {
	let op1: Option<OutPoint> =
		input.prevout.as_ref().map(|op| op.parse().map_err(TxError::PrevoutParse)).transpose()?;

	let op2 = match input.txid {
		Some(txid) => match input.vout {
			Some(vout) => Some(OutPoint {
				txid,
				vout,
			}),
			None => return Err(TxError::MissingVout),
		},
		None => None,
	};

	match (op1, op2) {
		(Some(op1), Some(op2)) => {
			if op1 != op2 {
				return Err(TxError::ConflictingPrevout);
			}
			Ok(op1)
		}
		(Some(op), None) => Ok(op),
		(None, Some(op)) => Ok(op),
		(None, None) => Err(TxError::NoPrevout),
	}
}

fn bytes_32(bytes: &[u8]) -> Option<[u8; 32]> {
	if bytes.len() != 32 {
		None
	} else {
		let mut array = [0; 32];
		for (x, y) in bytes.iter().zip(array.iter_mut()) {
			*y = *x;
		}
		Some(array)
	}
}

fn create_confidential_value(info: ConfidentialValueInfo) -> Result<confidential::Value, TxError> {
	Ok(match info.type_ {
		ConfidentialType::Null => confidential::Value::Null,
		ConfidentialType::Explicit => {
			confidential::Value::Explicit(info.value.ok_or(TxError::MissingField {
				field: "value".to_string(),
			})?)
		}
		ConfidentialType::Confidential => {
			let comm = PedersenCommitment::from_slice(
				&info
					.commitment
					.ok_or(TxError::MissingField {
						field: "commitment".to_string(),
					})?
					.0[..],
			)
			.map_err(TxError::ConfidentialCommitment)?;
			confidential::Value::Confidential(comm)
		}
	})
}

fn create_confidential_asset(info: ConfidentialAssetInfo) -> Result<confidential::Asset, TxError> {
	Ok(match info.type_ {
		ConfidentialType::Null => confidential::Asset::Null,
		ConfidentialType::Explicit => {
			confidential::Asset::Explicit(info.asset.ok_or(TxError::MissingField {
				field: "asset".to_string(),
			})?)
		}
		ConfidentialType::Confidential => {
			let gen = Generator::from_slice(
				&info
					.commitment
					.ok_or(TxError::MissingField {
						field: "commitment".to_string(),
					})?
					.0[..],
			)
			.map_err(TxError::ConfidentialCommitment)?;
			confidential::Asset::Confidential(gen)
		}
	})
}

fn create_confidential_nonce(info: ConfidentialNonceInfo) -> Result<confidential::Nonce, TxError> {
	Ok(match info.type_ {
		ConfidentialType::Null => confidential::Nonce::Null,
		ConfidentialType::Explicit => confidential::Nonce::Explicit(
			bytes_32(
				&info
					.nonce
					.ok_or(TxError::MissingField {
						field: "nonce".to_string(),
					})?
					.0[..],
			)
			.ok_or(TxError::NonceSize)?,
		),
		ConfidentialType::Confidential => {
			let pubkey = PublicKey::from_slice(
				&info
					.commitment
					.ok_or(TxError::MissingField {
						field: "commitment".to_string(),
					})?
					.0[..],
			)
			.map_err(TxError::ConfidentialCommitmentPublicKey)?;
			confidential::Nonce::Confidential(pubkey)
		}
	})
}

fn create_asset_issuance(info: AssetIssuanceInfo) -> Result<AssetIssuance, TxError> {
	Ok(AssetIssuance {
		asset_blinding_nonce: Tweak::from_slice(
			&info
				.asset_blinding_nonce
				.ok_or(TxError::MissingField {
					field: "asset_blinding_nonce".to_string(),
				})?
				.0[..],
		)
		.map_err(TxError::AssetBlindingNonce)?,
		asset_entropy: bytes_32(
			&info
				.asset_entropy
				.ok_or(TxError::MissingField {
					field: "asset_entropy".to_string(),
				})?
				.0[..],
		)
		.ok_or(TxError::AssetEntropySize)?,
		amount: create_confidential_value(info.amount.ok_or(TxError::MissingField {
			field: "amount".to_string(),
		})?)?,
		inflation_keys: create_confidential_value(info.inflation_keys.ok_or(
			TxError::MissingField {
				field: "inflation_keys".to_string(),
			},
		)?)?,
	})
}

fn create_script_sig(ss: InputScriptInfo) -> Result<Script, TxError> {
	if let Some(hex) = ss.hex {
		Ok(hex.0.into())
	} else if ss.asm.is_some() {
		Err(TxError::AsmNotSupported)
	} else {
		Err(TxError::NoScriptSig)
	}
}

fn create_pegin_witness(
	pd: PeginDataInfo,
	prevout: bitcoin::OutPoint,
) -> Result<Vec<Vec<u8>>, TxError> {
	let parsed_outpoint: bitcoin::OutPoint = pd.outpoint.parse().map_err(TxError::PeginOutpoint)?;

	if prevout != parsed_outpoint {
		return Err(TxError::PeginOutpointMismatch);
	}

	let asset = match create_confidential_asset(pd.asset)? {
		confidential::Asset::Explicit(asset) => asset,
		_ => return Err(TxError::PeginAssetNotExplicit),
	};

	Ok(vec![
		serialize(&pd.value),
		serialize(&asset),
		pd.genesis_hash.to_byte_array().to_vec(),
		serialize(&pd.claim_script.0),
		serialize(&pd.mainchain_tx_hex.0),
		serialize(&pd.merkle_proof.0),
	])
}

fn convert_outpoint_to_btc(p: elements::OutPoint) -> bitcoin::OutPoint {
	bitcoin::OutPoint {
		txid: bitcoin::Txid::from_byte_array(p.txid.to_byte_array()),
		vout: p.vout,
	}
}

fn create_input_witness(
	info: Option<InputWitnessInfo>,
	pd: Option<PeginDataInfo>,
	prevout: OutPoint,
) -> Result<TxInWitness, TxError> {
	let pegin_witness =
		if let Some(info_wit) = info.as_ref().and_then(|info| info.pegin_witness.as_ref()) {
			info_wit.iter().map(|h| h.clone().0).collect()
		} else if let Some(pd) = pd {
			create_pegin_witness(pd, convert_outpoint_to_btc(prevout))?
		} else {
			Default::default()
		};

	if let Some(wi) = info {
		Ok(TxInWitness {
			amount_rangeproof: wi
				.amount_rangeproof
				.map(|b| {
					RangeProof::from_slice(&b.0).map(|rp| Box::new(rp)).map_err(TxError::RangeProof)
				})
				.transpose()?,
			inflation_keys_rangeproof: wi
				.inflation_keys_rangeproof
				.map(|b| {
					RangeProof::from_slice(&b.0).map(|rp| Box::new(rp)).map_err(TxError::RangeProof)
				})
				.transpose()?,
			script_witness: match wi.script_witness {
				Some(ref w) => w.iter().map(|h| h.clone().0).collect(),
				None => Vec::new(),
			},
			pegin_witness,
		})
	} else {
		Ok(TxInWitness {
			pegin_witness,
			..Default::default()
		})
	}
}

fn create_input(input: InputInfo) -> Result<TxIn, TxError> {
	let has_issuance = input.has_issuance.unwrap_or(input.asset_issuance.is_some());
	let is_pegin = input.is_pegin.unwrap_or(input.pegin_data.is_some());
	let prevout = outpoint_from_input_info(&input)?;

	Ok(TxIn {
		previous_output: prevout,
		script_sig: input.script_sig.map(create_script_sig).transpose()?.unwrap_or_default(),
		sequence: elements::Sequence::from_height(
			input.sequence.unwrap_or_default().try_into().map_err(TxError::Sequence)?,
		),
		is_pegin,
		asset_issuance: if has_issuance {
			input.asset_issuance.map(create_asset_issuance).transpose()?.unwrap_or_default()
		} else {
			Default::default()
		},
		witness: create_input_witness(input.witness, input.pegin_data, prevout)?,
	})
}

fn create_script_pubkey(
	spk: OutputScriptInfo,
	used_network: &mut Option<Network>,
) -> Result<Script, TxError> {
	if let Some(hex) = spk.hex {
		Ok(hex.0.into())
	} else if spk.asm.is_some() {
		Err(TxError::AsmNotSupported)
	} else if let Some(address) = spk.address {
		// Error if another network had already been used.
		if let Some(network) = Network::from_params(address.params) {
			if used_network.replace(network).unwrap_or(network) != network {
				return Err(TxError::MixedNetworks);
			}
		}

		Ok(address.script_pubkey())
	} else {
		Err(TxError::NoScriptPubKey)
	}
}

fn create_bitcoin_script_pubkey(
	spk: hal::tx::OutputScriptInfo,
) -> Result<bitcoin::ScriptBuf, TxError> {
	if let Some(hex) = spk.hex {
		Ok(hex.0.into())
	} else if spk.asm.is_some() {
		Err(TxError::AsmNotSupported)
	} else if let Some(address) = spk.address {
		Ok(address.assume_checked().script_pubkey())
	} else {
		Err(TxError::NoScriptPubKey)
	}
}

fn create_output_witness(w: OutputWitnessInfo) -> Result<TxOutWitness, TxError> {
	Ok(TxOutWitness {
		surjection_proof: w
			.surjection_proof
			.map(|b| {
				SurjectionProof::from_slice(&b.0[..])
					.map(|sp| Box::new(sp))
					.map_err(TxError::SurjectionProof)
			})
			.transpose()?,
		rangeproof: w
			.rangeproof
			.map(|b| {
				RangeProof::from_slice(&b.0[..]).map(|rp| Box::new(rp)).map_err(TxError::RangeProof)
			})
			.transpose()?,
	})
}

fn create_script_pubkey_from_pegout_data(pd: PegoutDataInfo) -> Result<Script, TxError> {
	let mut builder = elements::script::Builder::new()
		.push_opcode(elements::opcodes::all::OP_RETURN)
		.push_slice(&pd.genesis_hash.to_byte_array())
		.push_slice(create_bitcoin_script_pubkey(pd.script_pub_key)?.as_bytes());
	for d in pd.extra_data {
		builder = builder.push_slice(&d.0);
	}
	Ok(builder.into_script())
}

fn create_output(output: OutputInfo) -> Result<TxOut, TxError> {
	let mut used_network = None;
	let value = output
		.value
		.ok_or(TxError::MissingField {
			field: "value".to_string(),
		})
		.and_then(create_confidential_value)?;
	let asset = output
		.asset
		.ok_or(TxError::MissingField {
			field: "asset".to_string(),
		})
		.and_then(create_confidential_asset)?;

	Ok(TxOut {
		asset,
		value,
		nonce: output
			.nonce
			.map(create_confidential_nonce)
			.transpose()?
			.unwrap_or(confidential::Nonce::Null),
		script_pubkey: if let Some(spk) = output.script_pub_key {
			create_script_pubkey(spk, &mut used_network)?
		} else if let Some(pd) = output.pegout_data {
			match value {
				confidential::Value::Explicit(v) => {
					if v != pd.value {
						return Err(TxError::PegoutValueMismatch);
					}
				}
				_ => return Err(TxError::PegoutValueNotExplicit),
			}
			if asset != create_confidential_asset(pd.asset.clone())? {
				return Err(TxError::PegoutAssetMismatch);
			}
			create_script_pubkey_from_pegout_data(pd)?
		} else {
			Default::default()
		},
		witness: output.witness.map(create_output_witness).transpose()?.unwrap_or_default(),
	})
}

fn parse_network(s: &str) -> Result<Network, TxError> {
	match s.to_lowercase().as_str() {
		"liquid" => Ok(Network::Liquid),
		"liquid-testnet" | "liquidtestnet" => Ok(Network::LiquidTestnet),
		"elementsregtest" | "elements-regtest" | "regtest" => Ok(Network::ElementsRegtest),
		_ => Err(TxError::NetworkParse(format!("unknown network: {}", s))),
	}
}
