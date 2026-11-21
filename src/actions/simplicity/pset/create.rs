// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use std::collections::HashMap;

use elements::confidential;
use elements::pset::PartiallySignedTransaction;
use elements::{Address, AssetId, OutPoint, Transaction, TxIn, TxOut, Txid};
use serde::Deserialize;

use super::{PsetError, UpdatedPset};

#[derive(Debug, thiserror::Error)]
pub enum PsetCreateError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("invalid inputs JSON: {0}")]
	InputsJsonParse(serde_json::Error),

	#[error("invalid outputs JSON: {0}")]
	OutputsJsonParse(serde_json::Error),

	#[error("invalid amount: {0}")]
	AmountParse(elements::bitcoin::amount::ParseAmountError),

	#[error("invalid address: {0}")]
	AddressParse(elements::address::AddressError),

	#[error("confidential addresses are not yet supported")]
	ConfidentialAddressNotSupported,

	#[error("invalid OP_RETURN hex data: {0}")]
	OpReturnHexParse(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(untagged, deny_unknown_fields)]
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
				// Use liquid bitcoin asset as default for map format
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

/// Create an empty PSET
pub fn pset_create(inputs_json: &str, outputs_json: &str) -> Result<UpdatedPset, PsetCreateError> {
	// Parse inputs JSON
	let input_specs: Vec<InputSpec> =
		serde_json::from_str(inputs_json).map_err(PsetCreateError::InputsJsonParse)?;

	// Parse outputs JSON - support both array and map formats
	let output_specs: Vec<OutputSpec> =
		serde_json::from_str(outputs_json).map_err(PsetCreateError::OutputsJsonParse)?;

	// Create transaction inputs
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

	// Create transaction outputs
	let mut outputs = Vec::new();
	for output_spec in output_specs.into_iter().flat_map(OutputSpec::flatten) {
		let output_spec = output_spec?; // serde has crappy error messages so we defer parsing and then have to unwrap errors

		let script_pubkey = match output_spec.address.as_str() {
			"fee" => elements::Script::new(),
			x if x.starts_with("data:") => {
				// OP_RETURN output: "data:HEXDATA"
				let hex_data = &x[5..];
				let data = hex::decode(hex_data)
					.map_err(|e| PsetCreateError::OpReturnHexParse(e.to_string()))?;
				elements::script::Builder::new()
					.push_opcode(elements::opcodes::all::OP_RETURN)
					.push_slice(&data)
					.into_script()
			}
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

	// Create the transaction
	let tx = Transaction {
		version: 2,
		lock_time: elements::LockTime::ZERO,
		input: inputs,
		output: outputs,
	};

	// Create PSET from transaction
	let pset = PartiallySignedTransaction::from_tx(tx);

	Ok(UpdatedPset {
		pset: pset.to_string(),
		updated_values: vec![
			// FIXME we technically update a whole slew of fields; see the implementation
			// of PartiallySignedTransaction::from_tx. Should we attempt to exhaustively
			// list them here? Or list none? Or what?
		],
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	const TXID: &str = "0000000000000000000000000000000000000000000000000000000000000000";
	const ASSET: &str = "0000000000000000000000000000000000000000000000000000000000000000";
	const ADDRESS: &str = "ert1qgqyvtapw3hp7p9anwf580rz4z0p4v9dy203prh";

	#[test]
	fn pset_create_rejects_unknown_input_fields() {
		for value in [json!(1), json!("ignored"), json!(null)] {
			let inputs = json!([{"txid": TXID, "vout": 1, "sequnce": value}]);
			match pset_create(&inputs.to_string(), "[]") {
				Err(PsetCreateError::InputsJsonParse(error)) => {
					assert!(error.to_string().contains("unknown field `sequnce`"));
				}
				_ => panic!("expected unknown input field to be rejected: {inputs}"),
			}
		}
	}

	#[test]
	fn pset_create_rejects_unknown_output_fields() {
		for value in [json!(1), json!("ignored"), json!(null)] {
			let outputs = json!([{
				"address": "fee", "asset": ASSET, "amount": 0.5, "ammount": value,
			}]);
			assert!(
				matches!(
					pset_create("[]", &outputs.to_string()),
					Err(PsetCreateError::OutputsJsonParse(_))
				),
				"expected unknown output field to be rejected: {outputs}"
			);
		}
	}

	#[test]
	fn pset_create_preserves_input_sequences() {
		let inputs = json!([
			{"txid": TXID, "vout": 1},
			{"txid": TXID, "vout": 2, "sequence": null},
			{"txid": TXID, "vout": 3, "sequence": 0xfffffffe_u32},
		]);
		let result = pset_create(&inputs.to_string(), "[]").unwrap();
		let pset: PartiallySignedTransaction = result.pset.parse().unwrap();
		let tx = pset.extract_tx().unwrap();
		assert_eq!(tx.input.len(), 3);
		for (index, sequence) in [0xffffffff, 0xffffffff, 0xfffffffe].into_iter().enumerate() {
			assert_eq!(tx.input[index].previous_output.txid.to_string(), TXID);
			assert_eq!(tx.input[index].previous_output.vout, index as u32 + 1);
			assert_eq!(tx.input[index].sequence.to_consensus_u32(), sequence);
		}
	}

	#[test]
	fn pset_create_preserves_output_formats() {
		let outputs = json!([
			{"address": ADDRESS, "asset": ASSET, "amount": 0.5},
			{(ADDRESS): 0.5},
			{"address": "fee", "asset": ASSET, "amount": 0.25},
			{"fee": 0.25},
			{"address": "data:abcd", "asset": ASSET, "amount": 0.0},
			{"data:abcd": 0.0},
		]);
		let result = pset_create("[]", &outputs.to_string()).unwrap();
		let pset: PartiallySignedTransaction = result.pset.parse().unwrap();
		let tx = pset.extract_tx().unwrap();
		assert_eq!(tx.output.len(), 6);
		let expected_script = ADDRESS.parse::<Address>().unwrap().script_pubkey();
		for (pair, (script, amount)) in tx.output.chunks_exact(2).zip([
			(expected_script, 50_000_000),
			(elements::Script::new(), 25_000_000),
			(elements::Script::from(vec![0x6a, 0x02, 0xab, 0xcd]), 0),
		]) {
			for output in pair {
				assert_eq!(output.script_pubkey, script);
				assert_eq!(output.value.explicit(), Some(amount));
			}
		}
	}
}
