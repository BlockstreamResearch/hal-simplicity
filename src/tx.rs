use elements::encode::serialize;
use elements::secp256k1_zkp::{RangeProof, SurjectionProof};
use elements::{
	bitcoin, confidential, Address, AssetIssuance, PeginData, PegoutData, Script, Transaction,
	TxIn, TxInWitness, TxOut, TxOutWitness, Txid, Wtxid,
};

use serde::{Deserialize, Serialize};

use crate::{GetInfo, HexBytes, Network};

use crate::confidential::{ConfidentialAssetInfo, ConfidentialNonceInfo, ConfidentialValueInfo};

const BTCNET: elements::bitcoin::Network = elements::bitcoin::Network::Bitcoin;

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct AssetIssuanceInfo {
	pub asset_blinding_nonce: Option<HexBytes>,
	pub asset_entropy: Option<HexBytes>,
	pub amount: Option<ConfidentialValueInfo>,
	pub inflation_keys: Option<ConfidentialValueInfo>,
}

impl GetInfo<AssetIssuanceInfo> for AssetIssuance {
	fn get_info(&self, network: Network) -> AssetIssuanceInfo {
		AssetIssuanceInfo {
			asset_blinding_nonce: Some(self.asset_blinding_nonce[..].into()),
			asset_entropy: Some(self.asset_entropy[..].into()),
			amount: Some(self.amount.get_info(network)),
			inflation_keys: Some(self.inflation_keys.get_info(network)),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct PeginDataInfo {
	pub outpoint: String,
	pub value: u64,
	pub asset: ConfidentialAssetInfo,
	pub genesis_hash: bitcoin::BlockHash,
	pub claim_script: HexBytes,
	pub mainchain_tx_hex: HexBytes,
	pub mainchain_tx: Option<hal::tx::TransactionInfo>,
	pub merkle_proof: HexBytes,
	pub referenced_block: bitcoin::BlockHash,
}

impl<'tx> GetInfo<PeginDataInfo> for PeginData<'tx> {
	fn get_info(&self, network: Network) -> PeginDataInfo {
		PeginDataInfo {
			outpoint: self.outpoint.to_string(),
			value: self.value,
			asset: self.asset.get_info(network),
			genesis_hash: self.genesis_hash,
			claim_script: self.claim_script.into(),
			mainchain_tx_hex: self.tx.into(),
			mainchain_tx: match bitcoin::consensus::encode::deserialize::<bitcoin::Transaction>(
				self.tx,
			) {
				Ok(tx) => Some(hal::GetInfo::get_info(&tx, BTCNET)),
				Err(_) => None,
			},
			merkle_proof: self.merkle_proof.into(),
			referenced_block: self.referenced_block,
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct InputWitnessInfo {
	pub amount_rangeproof: Option<HexBytes>,
	pub inflation_keys_rangeproof: Option<HexBytes>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub script_witness: Option<Vec<HexBytes>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pegin_witness: Option<Vec<HexBytes>>,
}

impl GetInfo<InputWitnessInfo> for TxInWitness {
	fn get_info(&self, _network: Network) -> InputWitnessInfo {
		InputWitnessInfo {
			amount_rangeproof: self
				.amount_rangeproof
				.as_ref()
				.map(|r| RangeProof::serialize(r).into()),
			inflation_keys_rangeproof: self
				.inflation_keys_rangeproof
				.as_ref()
				.map(|r| RangeProof::serialize(r).into()),
			script_witness: if !self.script_witness.is_empty() {
				Some(self.script_witness.iter().map(|w| w.clone().into()).collect())
			} else {
				None
			},
			pegin_witness: if !self.pegin_witness.is_empty() {
				Some(self.pegin_witness.iter().map(|w| w.clone().into()).collect())
			} else {
				None
			},
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct InputScriptInfo {
	pub hex: Option<HexBytes>,
	pub asm: Option<String>,
}

pub struct InputScript<'a>(pub &'a Script);

impl<'a> GetInfo<InputScriptInfo> for InputScript<'a> {
	fn get_info(&self, _network: Network) -> InputScriptInfo {
		InputScriptInfo {
			hex: Some(self.0.to_bytes().into()),
			asm: Some(self.0.asm()),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct InputInfo {
	pub prevout: Option<String>,
	pub txid: Option<Txid>,
	pub vout: Option<u32>,
	pub script_sig: Option<InputScriptInfo>,
	pub sequence: Option<u32>,

	pub is_pegin: Option<bool>,
	pub has_issuance: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub asset_issuance: Option<AssetIssuanceInfo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub witness: Option<InputWitnessInfo>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub pegin_data: Option<PeginDataInfo>,
}

impl GetInfo<InputInfo> for TxIn {
	fn get_info(&self, network: Network) -> InputInfo {
		InputInfo {
			// fmt::Display on elements outpoints show the `[elements]` prefix
			prevout: Some(format!("{}:{}", self.previous_output.txid, self.previous_output.vout)),
			txid: Some(self.previous_output.txid),
			vout: Some(self.previous_output.vout),
			sequence: Some(self.sequence.to_consensus_u32()),
			script_sig: Some(GetInfo::get_info(&InputScript(&self.script_sig), network)),

			is_pegin: Some(self.is_pegin),
			has_issuance: Some(self.has_issuance()),
			asset_issuance: if self.has_issuance() {
				Some(self.asset_issuance.get_info(network))
			} else {
				None
			},
			witness: if !self.witness.is_empty() {
				Some(self.witness.get_info(network))
			} else {
				None
			},
			pegin_data: self.pegin_data().map(|p| p.get_info(network)),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct PegoutDataInfo {
	pub value: u64,
	pub asset: ConfidentialAssetInfo,
	pub genesis_hash: bitcoin::BlockHash,
	pub script_pub_key: hal::tx::OutputScriptInfo,
	pub extra_data: Vec<HexBytes>,
}

impl<'tx> GetInfo<PegoutDataInfo> for PegoutData<'tx> {
	fn get_info(&self, network: Network) -> PegoutDataInfo {
		PegoutDataInfo {
			value: self.value,
			asset: self.asset.get_info(network),
			genesis_hash: self.genesis_hash,
			script_pub_key: hal::GetInfo::get_info(
				&hal::tx::OutputScript(&self.script_pubkey),
				BTCNET,
			),
			extra_data: self.extra_data.iter().map(|w| HexBytes::from(*w)).collect(),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OutputWitnessInfo {
	pub surjection_proof: Option<HexBytes>,
	pub rangeproof: Option<HexBytes>,
}

impl GetInfo<OutputWitnessInfo> for TxOutWitness {
	fn get_info(&self, _network: Network) -> OutputWitnessInfo {
		OutputWitnessInfo {
			surjection_proof: self
				.surjection_proof
				.as_ref()
				.map(|p| SurjectionProof::serialize(p).into()),
			rangeproof: self.rangeproof.as_ref().map(|p| RangeProof::serialize(p).into()),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OutputScriptInfo {
	pub hex: Option<HexBytes>,
	pub asm: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none", rename = "type")]
	pub type_: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub address: Option<Address>,
}

pub struct OutputScript<'a>(pub &'a Script);

impl<'a> GetInfo<OutputScriptInfo> for OutputScript<'a> {
	fn get_info(&self, network: Network) -> OutputScriptInfo {
		OutputScriptInfo {
			hex: Some(self.0.to_bytes().into()),
			asm: Some(self.0.asm()),
			type_: Some(
				if self.0.is_p2pk() {
					"p2pk"
				} else if self.0.is_p2pkh() {
					"p2pkh"
				} else if self.0.is_op_return() {
					"opreturn"
				} else if self.0.is_p2sh() {
					"p2sh"
				} else if self.0.is_v0_p2wpkh() {
					"p2wpkh"
				} else if self.0.is_v0_p2wsh() {
					"p2wsh"
				} else {
					"unknown"
				}
				.to_owned(),
			),
			address: Address::from_script(self.0, None, network.address_params()),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OutputInfo {
	pub script_pub_key: Option<OutputScriptInfo>,

	pub asset: Option<ConfidentialAssetInfo>,
	pub value: Option<ConfidentialValueInfo>,
	pub nonce: Option<ConfidentialNonceInfo>,
	pub witness: Option<OutputWitnessInfo>,
	pub is_fee: Option<bool>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub pegout_data: Option<PegoutDataInfo>,
}

impl GetInfo<OutputInfo> for TxOut {
	fn get_info(&self, network: Network) -> OutputInfo {
		let is_fee = {
			// An output is fee if both the asset and the value are explicit
			// and if the output script is empty.
			let exp_ass = matches!(self.asset, confidential::Asset::Explicit(_));
			let exp_val = matches!(self.value, confidential::Value::Explicit(_));

			exp_ass && exp_val && self.script_pubkey.is_empty()
		};

		OutputInfo {
			script_pub_key: Some(GetInfo::get_info(&OutputScript(&self.script_pubkey), network)),
			asset: Some(self.asset.get_info(network)),
			value: Some(self.value.get_info(network)),
			nonce: Some(self.nonce.get_info(network)),
			witness: Some(self.witness.get_info(network)),
			is_fee: Some(is_fee),
			pegout_data: self.pegout_data().map(|p| p.get_info(network)),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct TransactionInfo {
	pub txid: Option<Txid>,
	pub wtxid: Option<Wtxid>,
	pub hash: Option<Wtxid>,
	pub size: Option<usize>,
	pub weight: Option<usize>,
	pub vsize: Option<usize>,
	pub version: Option<u32>,
	pub locktime: Option<elements::LockTime>,
	pub inputs: Option<Vec<InputInfo>>,
	pub outputs: Option<Vec<OutputInfo>>,
}

impl GetInfo<TransactionInfo> for Transaction {
	fn get_info(&self, network: Network) -> TransactionInfo {
		TransactionInfo {
			txid: Some(self.txid()),
			wtxid: Some(self.wtxid()),
			hash: Some(self.wtxid()),
			version: Some(self.version),
			locktime: Some(self.lock_time),
			size: Some(serialize(self).len()),
			weight: Some(self.weight()),
			vsize: Some(self.weight() / 4),
			inputs: Some(self.input.iter().map(|i| i.get_info(network)).collect()),
			outputs: Some(self.output.iter().map(|o| o.get_info(network)).collect()),
		}
	}
}
