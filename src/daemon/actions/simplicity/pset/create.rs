use super::PsetError;
use crate::daemon::actions::types::{PsetCreateRequest, PsetCreateResponse};
use elements::confidential;
use elements::pset::PartiallySignedTransaction;
use elements::{Address, AssetId, OutPoint, Transaction, TxIn, TxOut, Txid};
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetCreateError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("Failed to parse inputs JSON: {0}")]
	InputsJsonParse(serde_json::Error),

	#[error("Failed to parse outputs JSON: {0}")]
	OutputsJsonParse(serde_json::Error),

	#[error("Failed to parse amount: {0}")]
	AmountParse(elements::bitcoin::amount::ParseAmountError),

	#[error("Failed to parse address: {0}")]
	AddressParse(elements::address::AddressError),

	#[error("Confidential addresses are not yet supported")]
	ConfidentialAddressNotSupported,
}

#[derive(Deserialize)]
struct InputSpec {
	txid: Txid,
	vout: u32,
	#[serde(default)]
	sequence: Option<u32>,
}

#[derive(Deserialize)]
struct FlattenedOutputSpec {
	address: String,
	asset: AssetId,
	#[serde(with = "elements::bitcoin::amount::serde::as_btc")]
	amount: elements::bitcoin::Amount,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OutputSpec {
	Explicit {
		address: String,
		asset: AssetId,
		#[serde(with = "elements::bitcoin::amount::serde::as_btc")]
		amount: elements::bitcoin::Amount,
	},
	Map(HashMap<String, f64>),
}

impl OutputSpec {
	fn flatten(self) -> Box<dyn Iterator<Item = Result<FlattenedOutputSpec, PsetCreateError>>> {
		match self {
			Self::Map(map) => Box::new(map.into_iter().map(|(address, amount)| {
				let default_asset = AssetId::from_slice(&[
					0x49, 0x9a, 0x81, 0x85, 0x45, 0xf6, 0xba, 0xe3, 0x9f, 0xc0, 0x3b, 0x63, 0x7f,
					0x2a, 0x4e, 0x1e, 0x64, 0xe5, 0x90, 0xca, 0xc1, 0xbc, 0x3a, 0x6f, 0x6d, 0x71,
					0xaa, 0x44, 0x43, 0x65, 0x4c, 0x14,
				])
				.expect("valid asset id");

				Ok(FlattenedOutputSpec {
					address,
					asset: default_asset,
					amount: elements::bitcoin::Amount::from_btc(amount)
						.map_err(PsetCreateError::AmountParse)?,
				})
			})),
			Self::Explicit {
				address,
				asset,
				amount,
			} => Box::new(
				Some(Ok(FlattenedOutputSpec {
					address,
					asset,
					amount,
				}))
				.into_iter(),
			),
		}
	}
}

pub fn create(req: PsetCreateRequest) -> Result<PsetCreateResponse, PsetCreateError> {
	let input_specs: Vec<InputSpec> =
		serde_json::from_str(&req.inputs).map_err(PsetCreateError::InputsJsonParse)?;

	let output_specs: Vec<OutputSpec> =
		serde_json::from_str(&req.outputs).map_err(PsetCreateError::OutputsJsonParse)?;

	let mut inputs = Vec::new();
	for input_spec in &input_specs {
		let outpoint = OutPoint::new(input_spec.txid, input_spec.vout);
		let sequence = elements::Sequence(input_spec.sequence.unwrap_or(0xffffffff));

		inputs.push(TxIn {
			previous_output: outpoint,
			script_sig: elements::Script::new(),
			sequence,
			asset_issuance: Default::default(),
			witness: Default::default(),
			is_pegin: false,
		});
	}

	let mut outputs = Vec::new();
	for output_spec in output_specs.into_iter().flat_map(OutputSpec::flatten) {
		let output_spec = output_spec?;

		let script_pubkey = match output_spec.address.as_str() {
			"fee" => elements::Script::new(),
			x => {
				let addr = x.parse::<Address>().map_err(PsetCreateError::AddressParse)?;
				if addr.is_blinded() {
					return Err(PsetCreateError::ConfidentialAddressNotSupported);
				}
				addr.script_pubkey()
			}
		};

		outputs.push(TxOut {
			asset: confidential::Asset::Explicit(output_spec.asset),
			value: confidential::Value::Explicit(output_spec.amount.to_sat()),
			nonce: elements::confidential::Nonce::Null,
			script_pubkey,
			witness: elements::TxOutWitness::empty(),
		});
	}

	let tx = Transaction {
		version: 2,
		lock_time: elements::LockTime::ZERO,
		input: inputs,
		output: outputs,
	};

	let pset = PartiallySignedTransaction::from_tx(tx);

	Ok(PsetCreateResponse {
		pset: pset.to_string(),
		updated_values: vec![],
	})
}
