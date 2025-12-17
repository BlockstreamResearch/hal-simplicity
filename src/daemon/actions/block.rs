use elements::encode::{deserialize, serialize};
use elements::{dynafed, Block, BlockExtData, BlockHeader};
use thiserror::Error;

use crate::block::{BlockHeaderInfo, BlockInfo, ParamsInfo, ParamsType};
use crate::utils::{GetInfo, Network};

use super::types::{BlockCreateRequest, BlockDecodeRequest};

#[derive(Debug, Error)]
pub enum BlockError {
	#[error("Failed to parse block info JSON: {0}")]
	JsonParse(serde_json::Error),
	#[error("Cannot provide transactions both in JSON and raw")]
	ConflictingTransactions,

	#[error("No transactions provided")]
	NoTransactions,

	#[error("Failed to deserialize raw transaction: {0}")]
	TransactionDeserialize(super::tx::TxError),

	#[error("Failed to deserialize raw transaction using Elements: {0}")]
	ElementsTransactionDeserialize(elements::encode::Error),

	#[error("Failed to decode raw block hex: {0}")]
	BlockHex(hex::FromHexError),

	#[error("Invalid block format: {0}")]
	BlockDeserialize(elements::encode::Error),

	#[error("Failed to serialize response: {0}")]
	Serialize(String),

	#[error("Failed to parse network: {0}")]
	NetworkParse(String),

	#[error("Missing {field} in {context}")]
	MissingField {
		field: String,
		context: String,
	},
}

pub fn create(req: BlockCreateRequest) -> Result<String, BlockError> {
	let info = serde_json::from_str::<BlockInfo>(&req.block_info).map_err(BlockError::JsonParse)?;

	let block = Block {
		header: create_block_header(info.header)?,
		txdata: match (info.transactions, info.raw_transactions) {
			(Some(_), Some(_)) => return Err(BlockError::ConflictingTransactions),
			(None, None) => return Err(BlockError::NoTransactions),
			(Some(infos), None) => infos
				.into_iter()
				.map(|info| {
					super::tx::create_transaction(info).map_err(BlockError::TransactionDeserialize)
				})
				.collect::<Result<Vec<_>, _>>()?,
			(None, Some(raws)) => raws
				.into_iter()
				.map(|r| deserialize(&r.0).map_err(BlockError::ElementsTransactionDeserialize))
				.collect::<Result<Vec<_>, _>>()?,
		},
	};

	let block_bytes = serialize(&block);
	Ok(hex::encode(&block_bytes))
}

pub fn decode(req: BlockDecodeRequest) -> Result<serde_json::Value, BlockError> {
	let raw_block = hex::decode(&req.raw_block).map_err(BlockError::BlockHex)?;
	let network = req
		.network
		.as_ref()
		.map(|s| parse_network(s))
		.transpose()?
		.unwrap_or(Network::ElementsRegtest);
	if req.txids.unwrap_or(false) {
		let block: Block = deserialize(&raw_block).map_err(BlockError::BlockDeserialize)?;
		let info = BlockInfo {
			header: GetInfo::get_info(&block.header, network),
			txids: Some(block.txdata.iter().map(|t| t.txid()).collect()),
			transactions: None,
			raw_transactions: None,
		};
		serde_json::to_value(&info).map_err(|e| BlockError::Serialize(format!("{}", e)))
	} else {
		let header: BlockHeader = match deserialize(&raw_block) {
			Ok(header) => header,
			Err(_) => {
				let block: Block = deserialize(&raw_block).map_err(BlockError::BlockDeserialize)?;
				block.header
			}
		};
		let info = GetInfo::get_info(&header, network);
		serde_json::to_value(&info).map_err(|e| BlockError::Serialize(format!("{}", e)))
	}
}

fn create_params(info: ParamsInfo) -> Result<dynafed::Params, BlockError> {
	Ok(match info.params_type {
		ParamsType::Null => dynafed::Params::Null,
		ParamsType::Compact => dynafed::Params::Compact {
			signblockscript: info
				.signblockscript
				.ok_or_else(|| BlockError::MissingField {
					field: "signblockscript".to_string(),
					context: "compact params".to_string(),
				})?
				.0
				.into(),
			signblock_witness_limit: info.signblock_witness_limit.ok_or_else(|| {
				BlockError::MissingField {
					field: "signblock_witness_limit".to_string(),
					context: "compact params".to_string(),
				}
			})?,
			elided_root: info.elided_root.ok_or_else(|| BlockError::MissingField {
				field: "elided_root".to_string(),
				context: "compact params".to_string(),
			})?,
		},
		ParamsType::Full => dynafed::Params::Full(dynafed::FullParams::new(
			info.signblockscript
				.ok_or_else(|| BlockError::MissingField {
					field: "signblockscript".to_string(),
					context: "full params".to_string(),
				})?
				.0
				.into(),
			info.signblock_witness_limit.ok_or_else(|| BlockError::MissingField {
				field: "signblock_witness_limit".to_string(),
				context: "full params".to_string(),
			})?,
			info.fedpeg_program
				.ok_or_else(|| BlockError::MissingField {
					field: "fedpeg_program".to_string(),
					context: "full params".to_string(),
				})?
				.0
				.into(),
			info.fedpeg_script
				.ok_or_else(|| BlockError::MissingField {
					field: "fedpeg_script".to_string(),
					context: "full params".to_string(),
				})?
				.0,
			info.extension_space
				.ok_or_else(|| BlockError::MissingField {
					field: "extension_space".to_string(),
					context: "full params".to_string(),
				})?
				.into_iter()
				.map(|b| b.0)
				.collect(),
		)),
	})
}

fn create_block_header(info: BlockHeaderInfo) -> Result<BlockHeader, BlockError> {
	Ok(BlockHeader {
		version: info.version,
		prev_blockhash: info.previous_block_hash,
		merkle_root: info.merkle_root,
		time: info.time,
		height: info.height,
		ext: if info.dynafed {
			BlockExtData::Dynafed {
				current: create_params(info.dynafed_current.ok_or_else(|| {
					BlockError::MissingField {
						field: "dynafed_current".to_string(),
						context: "block header".to_string(),
					}
				})?)?,
				proposed: create_params(info.dynafed_proposed.ok_or_else(|| {
					BlockError::MissingField {
						field: "dynafed_proposed".to_string(),
						context: "block header".to_string(),
					}
				})?)?,
				signblock_witness: info
					.dynafed_witness
					.ok_or_else(|| BlockError::MissingField {
						field: "dynafed_witness".to_string(),
						context: "block header".to_string(),
					})?
					.into_iter()
					.map(|b| b.0)
					.collect(),
			}
		} else {
			BlockExtData::Proof {
				challenge: info
					.legacy_challenge
					.ok_or_else(|| BlockError::MissingField {
						field: "legacy_challenge".to_string(),
						context: "block header".to_string(),
					})?
					.0
					.into(),
				solution: info
					.legacy_solution
					.ok_or_else(|| BlockError::MissingField {
						field: "legacy_solution".to_string(),
						context: "block header".to_string(),
					})?
					.0
					.into(),
			}
		},
	})
}

fn parse_network(s: &str) -> Result<Network, BlockError> {
	match s.to_lowercase().as_str() {
		"liquid" => Ok(Network::Liquid),
		"liquid-testnet" | "liquidtestnet" => Ok(Network::LiquidTestnet),
		"elementsregtest" | "elements-regtest" | "regtest" => Ok(Network::ElementsRegtest),
		_ => Err(BlockError::NetworkParse(format!("unknown network: {}", s))),
	}
}
