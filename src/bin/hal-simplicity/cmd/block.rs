use std::io::Write;

use elements::encode::{deserialize, serialize};
use elements::{dynafed, Block, BlockExtData, BlockHeader};

use crate::cmd;
use crate::cmd::tx::create_transaction;
use hal_simplicity::block::{BlockHeaderInfo, BlockInfo, ParamsInfo, ParamsType};
use log::warn;

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

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("block", "manipulate blocks")
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
	cmd::subcommand("create", "create a raw block from JSON").args(&[
		cmd::arg("block-info", "the block info in JSON").required(false),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	])
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
	if info.block_hash.is_some() {
		warn!("Field \"block_hash\" is ignored.");
	}

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

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let info = serde_json::from_str::<BlockInfo>(&cmd::arg_or_stdin(matches, "block-info"))
		.map_err(BlockError::InvalidJsonInput)
		.unwrap_or_else(|e| panic!("{}", e));

	if info.txids.is_some() {
		warn!("Field \"txids\" is ignored.");
	}

	let create_block = || -> Result<Block, BlockError> {
		let header = create_block_header(info.header)?;
		let txdata = match (info.transactions, info.raw_transactions) {
			(Some(_), Some(_)) => return Err(BlockError::ConflictingTransactions),
			(None, None) => return Err(BlockError::NoTransactions),
			(Some(infos), None) => infos
				.into_iter()
				.map(create_transaction)
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
	};

	let block = create_block().unwrap_or_else(|e| panic!("{}", e));

	let block_bytes = serialize(&block);
	if matches.is_present("raw-stdout") {
		::std::io::stdout().write_all(&block_bytes).unwrap();
	} else {
		print!("{}", hex::encode(&block_bytes));
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw block to JSON").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::arg("raw-block", "the raw block in hex").required(false),
		cmd::opt("txids", "provide transactions IDs instead of full transactions"),
	])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>) {
	let hex_tx = cmd::arg_or_stdin(matches, "raw-block");
	let raw_tx = hex::decode(hex_tx.as_ref())
		.map_err(BlockError::CouldNotDecodeRawBlockHex)
		.unwrap_or_else(|e| panic!("{}", e));

	if matches.is_present("txids") {
		let block: Block = deserialize(&raw_tx)
			.map_err(BlockError::BlockDeserialize)
			.unwrap_or_else(|e| panic!("{}", e));
		let info = BlockInfo {
			header: crate::GetInfo::get_info(&block.header, cmd::network(matches)),
			txids: Some(block.txdata.iter().map(|t| t.txid()).collect()),
			transactions: None,
			raw_transactions: None,
		};
		cmd::print_output(matches, &info)
	} else {
		let header: BlockHeader = match deserialize(&raw_tx) {
			Ok(header) => header,
			Err(_) => {
				let block: Block = deserialize(&raw_tx)
					.map_err(BlockError::BlockDeserialize)
					.unwrap_or_else(|e| panic!("{}", e));
				block.header
			}
		};
		let info = crate::GetInfo::get_info(&header, cmd::network(matches));
		cmd::print_output(matches, &info)
	}
}
