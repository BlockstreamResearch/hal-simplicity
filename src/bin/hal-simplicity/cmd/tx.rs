use std::convert::TryInto;
use std::io::Write;

use clap;
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
use log::warn;

use crate::cmd;
use hal_simplicity::confidential::{
	ConfidentialAssetInfo, ConfidentialNonceInfo, ConfidentialType, ConfidentialValueInfo,
};
use hal_simplicity::tx::{
	AssetIssuanceInfo, InputInfo, InputScriptInfo, InputWitnessInfo, OutputInfo, OutputScriptInfo,
	OutputWitnessInfo, PeginDataInfo, PegoutDataInfo, TransactionInfo,
};
use hal_simplicity::Network;

#[derive(Debug, thiserror::Error)]
pub enum TxError {
	#[error("invalid JSON provided: {0}")]
	JsonParse(serde_json::Error),

	#[error("failed to decode raw transaction hex: {0}")]
	TxHex(hex::FromHexError),

	#[error("invalid tx format: {0}")]
	TxDeserialize(elements::encode::Error),

	#[error("field \"{field}\" is required.")]
	MissingField {
		field: String,
	},

	#[error("invalid prevout format: {0}")]
	PrevoutParse(bitcoin::blockdata::transaction::ParseOutPointError),

	#[error("txid field given without vout field")]
	MissingVout,

	#[error("conflicting prevout information")]
	ConflictingPrevout,

	#[error("no previous output provided")]
	NoPrevout,

	#[error("invalid confidential commitment: {0}")]
	ConfidentialCommitment(elements::secp256k1_zkp::Error),

	#[error("invalid confidential publicKey: {0}")]
	ConfidentialCommitmentPublicKey(secp256k1::Error),

	#[error("wrong size of nonce field")]
	NonceSize,

	#[error("invalid size of asset_entropy")]
	AssetEntropySize,

	#[error("invalid asset_blinding_nonce: {0}")]
	AssetBlindingNonce(elements::secp256k1_zkp::Error),

	#[error("decoding script assembly is not yet supported")]
	AsmNotSupported,

	#[error("no scriptSig info provided")]
	NoScriptSig,

	#[error("no scriptPubKey info provided")]
	NoScriptPubKey,

	#[error("invalid outpoint in pegin_data: {0}")]
	PeginOutpoint(bitcoin::blockdata::transaction::ParseOutPointError),

	#[error("outpoint in pegin_data does not correspond to input value")]
	PeginOutpointMismatch,

	#[error("asset in pegin_data should be explicit")]
	PeginAssetNotExplicit,

	#[error("invalid rangeproof: {0}")]
	RangeProof(elements::secp256k1_zkp::Error),

	#[error("invalid sequence: {0}")]
	Sequence(core::num::TryFromIntError),

	#[error("addresses for different networks are used in the output scripts")]
	MixedNetworks,

	#[error("invalid surjection proof: {0}")]
	SurjectionProof(elements::secp256k1_zkp::Error),

	#[error("value in pegout_data does not correspond to output value")]
	PegoutValueMismatch,

	#[error("explicit value is required for pegout data")]
	PegoutValueNotExplicit,

	#[error("asset in pegout_data does not correspond to output value")]
	PegoutAssetMismatch,
}

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("tx", "manipulate transactions")
		.subcommand(cmd_create())
		.subcommand(cmd_decode())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(m)) => exec_create(m),
		("decode", Some(m)) => exec_decode(m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_create<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create a raw transaction from JSON").args(&[
		cmd::arg("tx-info", "the transaction info in JSON").required(false),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	])
}

/// Check both ways to specify the outpoint and return error if conflicting.
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
	match info.type_ {
		ConfidentialType::Null => Ok(confidential::Value::Null),
		ConfidentialType::Explicit => {
			Ok(confidential::Value::Explicit(info.value.ok_or_else(|| TxError::MissingField {
				field: "value".to_string(),
			})?))
		}
		ConfidentialType::Confidential => {
			let commitment_data = info.commitment.ok_or_else(|| TxError::MissingField {
				field: "commitment".to_string(),
			})?;
			let comm = PedersenCommitment::from_slice(&commitment_data.0[..])
				.map_err(TxError::ConfidentialCommitment)?;
			Ok(confidential::Value::Confidential(comm))
		}
	}
}

fn create_confidential_asset(info: ConfidentialAssetInfo) -> Result<confidential::Asset, TxError> {
	match info.type_ {
		ConfidentialType::Null => Ok(confidential::Asset::Null),
		ConfidentialType::Explicit => {
			Ok(confidential::Asset::Explicit(info.asset.ok_or_else(|| TxError::MissingField {
				field: "asset".to_string(),
			})?))
		}
		ConfidentialType::Confidential => {
			let commitment_data = info.commitment.ok_or_else(|| TxError::MissingField {
				field: "commitment".to_string(),
			})?;
			let gen = Generator::from_slice(&commitment_data.0[..])
				.map_err(TxError::ConfidentialCommitment)?;
			Ok(confidential::Asset::Confidential(gen))
		}
	}
}

fn create_confidential_nonce(info: ConfidentialNonceInfo) -> Result<confidential::Nonce, TxError> {
	match info.type_ {
		ConfidentialType::Null => Ok(confidential::Nonce::Null),
		ConfidentialType::Explicit => {
			let nonce = info.nonce.ok_or_else(|| TxError::MissingField {
				field: "nonce".to_string(),
			})?;
			let bytes = bytes_32(&nonce.0[..]).ok_or(TxError::NonceSize)?;
			Ok(confidential::Nonce::Explicit(bytes))
		}
		ConfidentialType::Confidential => {
			let commitment_data = info.commitment.ok_or_else(|| TxError::MissingField {
				field: "commitment".to_string(),
			})?;
			let pubkey = PublicKey::from_slice(&commitment_data.0[..])
				.map_err(TxError::ConfidentialCommitmentPublicKey)?;
			Ok(confidential::Nonce::Confidential(pubkey))
		}
	}
}

fn create_asset_issuance(info: AssetIssuanceInfo) -> Result<AssetIssuance, TxError> {
	let asset_blinding_nonce_data =
		info.asset_blinding_nonce.ok_or_else(|| TxError::MissingField {
			field: "asset_blinding_nonce".to_string(),
		})?;
	let asset_blinding_nonce =
		Tweak::from_slice(&asset_blinding_nonce_data.0[..]).map_err(TxError::AssetBlindingNonce)?;

	let asset_entropy_data = info.asset_entropy.ok_or_else(|| TxError::MissingField {
		field: "asset_entropy".to_string(),
	})?;
	let asset_entropy = bytes_32(&asset_entropy_data.0[..]).ok_or(TxError::AssetEntropySize)?;

	let amount_info = info.amount.ok_or_else(|| TxError::MissingField {
		field: "amount".to_string(),
	})?;
	let amount = create_confidential_value(amount_info)?;

	let inflation_keys_info = info.inflation_keys.ok_or_else(|| TxError::MissingField {
		field: "inflation_keys".to_string(),
	})?;
	let inflation_keys = create_confidential_value(inflation_keys_info)?;

	Ok(AssetIssuance {
		asset_blinding_nonce,
		asset_entropy,
		amount,
		inflation_keys,
	})
}

fn create_script_sig(ss: InputScriptInfo) -> Result<Script, TxError> {
	if let Some(hex) = ss.hex {
		if ss.asm.is_some() {
			warn!("Field \"asm\" of input is ignored.");
		}
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
	let parsed_outpoint = pd.outpoint.parse().map_err(TxError::PeginOutpoint)?;
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
			if pd.is_some() {
				warn!("Field \"pegin_data\" of input is ignored.");
			}
			info_wit.iter().map(|h| h.clone().0).collect()
		} else if let Some(pd) = pd {
			create_pegin_witness(pd, convert_outpoint_to_btc(prevout))?
		} else {
			Default::default()
		};

	if let Some(wi) = info {
		let amount_rangeproof = wi
			.amount_rangeproof
			.map(|b| RangeProof::from_slice(&b.0).map_err(TxError::RangeProof).map(Box::new))
			.transpose()?;
		let inflation_keys_rangeproof = wi
			.inflation_keys_rangeproof
			.map(|b| RangeProof::from_slice(&b.0).map_err(TxError::RangeProof).map(Box::new))
			.transpose()?;

		Ok(TxInWitness {
			amount_rangeproof,
			inflation_keys_rangeproof,
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

	let script_sig = input.script_sig.map(create_script_sig).transpose()?.unwrap_or_default();

	let sequence = elements::Sequence::from_height(
		input.sequence.unwrap_or_default().try_into().map_err(TxError::Sequence)?,
	);

	let asset_issuance = if has_issuance {
		input.asset_issuance.map(create_asset_issuance).transpose()?.unwrap_or_default()
	} else {
		if input.asset_issuance.is_some() {
			warn!("Field \"asset_issuance\" of input is ignored.");
		}
		Default::default()
	};

	let witness = create_input_witness(input.witness, input.pegin_data, prevout)?;

	Ok(TxIn {
		previous_output: prevout,
		script_sig,
		sequence,
		is_pegin,
		asset_issuance,
		witness,
	})
}

fn create_script_pubkey(
	spk: OutputScriptInfo,
	used_network: &mut Option<Network>,
) -> Result<Script, TxError> {
	if spk.type_.is_some() {
		warn!("Field \"type\" of output is ignored.");
	}

	if let Some(hex) = spk.hex {
		if spk.asm.is_some() {
			warn!("Field \"asm\" of output is ignored.");
		}
		if spk.address.is_some() {
			warn!("Field \"address\" of output is ignored.");
		}

		//TODO(stevenroose) do script sanity check to avoid blackhole?
		Ok(hex.0.into())
	} else if spk.asm.is_some() {
		if spk.address.is_some() {
			warn!("Field \"address\" of output is ignored.");
		}
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
	if spk.type_.is_some() {
		warn!("Field \"type\" of output is ignored.");
	}

	if let Some(hex) = spk.hex {
		if spk.asm.is_some() {
			warn!("Field \"asm\" of output is ignored.");
		}
		if spk.address.is_some() {
			warn!("Field \"address\" of output is ignored.");
		}

		//TODO(stevenroose) do script sanity check to avoid blackhole?
		Ok(hex.0.into())
	} else if spk.asm.is_some() {
		if spk.address.is_some() {
			warn!("Field \"address\" of output is ignored.");
		}
		Err(TxError::AsmNotSupported)
	} else if let Some(address) = spk.address {
		Ok(address.assume_checked().script_pubkey())
	} else {
		Err(TxError::NoScriptPubKey)
	}
}

fn create_output_witness(w: OutputWitnessInfo) -> Result<TxOutWitness, TxError> {
	let surjection_proof = w
		.surjection_proof
		.map(|b| {
			SurjectionProof::from_slice(&b.0[..]).map_err(TxError::SurjectionProof).map(Box::new)
		})
		.transpose()?;
	let rangeproof = w
		.rangeproof
		.map(|b| RangeProof::from_slice(&b.0[..]).map_err(TxError::RangeProof).map(Box::new))
		.transpose()?;

	Ok(TxOutWitness {
		surjection_proof,
		rangeproof,
	})
}

fn create_script_pubkey_from_pegout_data(pd: PegoutDataInfo) -> Result<Script, TxError> {
	let script_pubkey = create_bitcoin_script_pubkey(pd.script_pub_key)?;
	let mut builder = elements::script::Builder::new()
		.push_opcode(elements::opcodes::all::OP_RETURN)
		.push_slice(&pd.genesis_hash.to_byte_array())
		.push_slice(script_pubkey.as_bytes());
	for d in pd.extra_data {
		builder = builder.push_slice(&d.0);
	}
	Ok(builder.into_script())
}

fn create_output(output: OutputInfo) -> Result<TxOut, TxError> {
	// Keep track of which network has been used in addresses and error if two different networks
	// are used.
	let mut used_network = None;
	let value_info = output.value.ok_or_else(|| TxError::MissingField {
		field: "value".to_string(),
	})?;
	let value = create_confidential_value(value_info)?;

	let asset_info = output.asset.ok_or_else(|| TxError::MissingField {
		field: "asset".to_string(),
	})?;
	let asset = create_confidential_asset(asset_info)?;

	let nonce = output
		.nonce
		.map(create_confidential_nonce)
		.transpose()?
		.unwrap_or(confidential::Nonce::Null);

	let script_pubkey = if let Some(spk) = output.script_pub_key {
		if output.pegout_data.is_some() {
			warn!("Field \"pegout_data\" of output is ignored.");
		}
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
		let pd_asset = create_confidential_asset(pd.asset.clone())?;
		if asset != pd_asset {
			return Err(TxError::PegoutAssetMismatch);
		}
		create_script_pubkey_from_pegout_data(pd)?
	} else {
		Default::default()
	};

	let witness = output.witness.map(create_output_witness).transpose()?.unwrap_or_default();

	Ok(TxOut {
		asset,
		value,
		nonce,
		script_pubkey,
		witness,
	})
}

pub fn create_transaction(info: TransactionInfo) -> Result<Transaction, TxError> {
	// Fields that are ignored.
	if info.txid.is_some() {
		warn!("Field \"txid\" is ignored.");
	}
	if info.hash.is_some() {
		warn!("Field \"hash\" is ignored.");
	}
	if info.size.is_some() {
		warn!("Field \"size\" is ignored.");
	}
	if info.weight.is_some() {
		warn!("Field \"weight\" is ignored.");
	}
	if info.vsize.is_some() {
		warn!("Field \"vsize\" is ignored.");
	}

	let version = info.version.ok_or_else(|| TxError::MissingField {
		field: "version".to_string(),
	})?;
	let lock_time = info.locktime.ok_or_else(|| TxError::MissingField {
		field: "locktime".to_string(),
	})?;

	let inputs = info
		.inputs
		.ok_or_else(|| TxError::MissingField {
			field: "inputs".to_string(),
		})?
		.into_iter()
		.map(create_input)
		.collect::<Result<Vec<_>, _>>()?;

	let outputs = info
		.outputs
		.ok_or_else(|| TxError::MissingField {
			field: "outputs".to_string(),
		})?
		.into_iter()
		.map(create_output)
		.collect::<Result<Vec<_>, _>>()?;

	Ok(Transaction {
		version,
		lock_time,
		input: inputs,
		output: outputs,
	})
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let info = serde_json::from_str::<TransactionInfo>(&cmd::arg_or_stdin(matches, "tx-info"))
		.map_err(TxError::JsonParse)
		.unwrap_or_else(|e| panic!("{}", e));
	let tx = create_transaction(info).unwrap_or_else(|e| panic!("{}", e));

	let tx_bytes = serialize(&tx);
	if matches.is_present("raw-stdout") {
		::std::io::stdout().write_all(&tx_bytes).unwrap();
	} else {
		print!("{}", hex::encode(&tx_bytes));
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw transaction to JSON")
		.args(&cmd::opts_networks())
		.args(&[cmd::opt_yaml(), cmd::arg("raw-tx", "the raw transaction in hex").required(false)])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>) {
	let hex_tx = cmd::arg_or_stdin(matches, "raw-tx");
	let raw_tx =
		hex::decode(hex_tx.as_ref()).map_err(TxError::TxHex).unwrap_or_else(|e| panic!("{}", e));
	let tx: Transaction =
		deserialize(&raw_tx).map_err(TxError::TxDeserialize).unwrap_or_else(|e| panic!("{}", e));

	let info = crate::GetInfo::get_info(&tx, cmd::network(matches));
	cmd::print_output(matches, &info)
}
