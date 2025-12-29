use elements::encode::deserialize;
use elements::{dynafed, Block, BlockExtData, BlockHeader};

use crate::block::{BlockHeaderInfo, BlockInfo, ParamsInfo, ParamsType};
use crate::Network;

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum BlockDecodeOutput {
	Info(BlockInfo),
	Header(BlockHeaderInfo),
}

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
	#[error("can't provide transactions both in JSON and raw.")]
	ConflictingTransactions,

	#[error("no transactions provided.")]
	NoTransactions,

	#[error("failed to deserialize transaction: {0}")]
	TransactionDeserialize(super::tx::TxError),

	#[error("invalid raw transaction: {0}")]
	InvalidRawTransaction(elements::encode::Error),

	#[error("invalid block format: {0}")]
	BlockDeserialize(elements::encode::Error),

	#[error("could not decode raw block hex: {0}")]
	CouldNotDecodeRawBlockHex(hex::FromHexError),

	#[error("invalid json JSON input: {0}")]
	InvalidJsonInput(serde_json::Error),

	#[error("{field} missing in {context}")]
	MissingField {
		field: String,
		context: String,
	},
}

fn create_params(info: ParamsInfo) -> Result<dynafed::Params, BlockError> {
	match info.params_type {
		ParamsType::Null => Ok(dynafed::Params::Null),
		ParamsType::Compact => Ok(dynafed::Params::Compact {
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
		}),
		ParamsType::Full => Ok(dynafed::Params::Full(dynafed::FullParams::new(
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
					field: "extension space".to_string(),
					context: "full params".to_string(),
				})?
				.into_iter()
				.map(|b| b.0)
				.collect(),
		))),
	}
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
						field: "current".to_string(),
						context: "dynafed params".to_string(),
					}
				})?)?,
				proposed: create_params(info.dynafed_proposed.ok_or_else(|| {
					BlockError::MissingField {
						field: "proposed".to_string(),
						context: "dynafed params".to_string(),
					}
				})?)?,
				signblock_witness: info
					.dynafed_witness
					.ok_or_else(|| BlockError::MissingField {
						field: "witness".to_string(),
						context: "dynafed params".to_string(),
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
						field: "challenge".to_string(),
						context: "proof params".to_string(),
					})?
					.0
					.into(),
				solution: info
					.legacy_solution
					.ok_or_else(|| BlockError::MissingField {
						field: "solution".to_string(),
						context: "proof params".to_string(),
					})?
					.0
					.into(),
			}
		},
	})
}

/// Create a block from block info.
pub fn block_create(info: BlockInfo) -> Result<Block, BlockError> {
	let header = create_block_header(info.header)?;
	let txdata = match (info.transactions, info.raw_transactions) {
		(Some(_), Some(_)) => return Err(BlockError::ConflictingTransactions),
		(None, None) => return Err(BlockError::NoTransactions),
		(Some(infos), None) => infos
			.into_iter()
			.map(super::tx::tx_create)
			.collect::<Result<Vec<_>, _>>()
			.map_err(BlockError::TransactionDeserialize)?,
		(None, Some(raws)) => raws
			.into_iter()
			.map(|r| deserialize(&r.0).map_err(BlockError::InvalidRawTransaction))
			.collect::<Result<Vec<_>, _>>()?,
	};
	Ok(Block {
		header,
		txdata,
	})
}

/// Decode a raw block and return block info or header info.
pub fn block_decode(
	raw_block_hex: &str,
	network: Network,
	txids_only: bool,
) -> Result<BlockDecodeOutput, BlockError> {
	use crate::GetInfo;

	let raw_block = hex::decode(raw_block_hex).map_err(BlockError::CouldNotDecodeRawBlockHex)?;

	if txids_only {
		let block: Block = deserialize(&raw_block).map_err(BlockError::BlockDeserialize)?;
		let info = BlockInfo {
			header: block.header.get_info(network),
			txids: Some(block.txdata.iter().map(|t| t.txid()).collect()),
			transactions: None,
			raw_transactions: None,
		};
		Ok(BlockDecodeOutput::Info(info))
	} else {
		let header: BlockHeader = match deserialize(&raw_block) {
			Ok(header) => header,
			Err(_) => {
				let block: Block = deserialize(&raw_block).map_err(BlockError::BlockDeserialize)?;
				block.header
			}
		};
		let info = header.get_info(network);
		Ok(BlockDecodeOutput::Header(info))
	}
}
