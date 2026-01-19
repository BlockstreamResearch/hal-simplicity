use std::str::FromStr;

use super::jsonrpc::{ErrorCode, JsonRpcService, RpcError, RpcHandler};
use serde_json::Value;

use super::types::*;
use crate::actions;

use crate::Network;

/// RPC method names
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcMethod {
	AddressCreate,
	AddressInspect,
	BlockCreate,
	BlockDecode,
	TxCreate,
	TxDecode,
	KeypairGenerate,
	SimplicityInfo,
	SimplicitySighash,
	PsetCreate,
	PsetExtract,
	PsetFinalize,
	PsetRun,
	PsetUpdateInput,
}

impl FromStr for RpcMethod {
	type Err = RpcError;

	fn from_str(s: &str) -> Result<Self, RpcError> {
		let method = match s {
			"address_create" => Self::AddressCreate,
			"address_inspect" => Self::AddressInspect,
			"block_create" => Self::BlockCreate,
			"block_decode" => Self::BlockDecode,
			"tx_create" => Self::TxCreate,
			"tx_decode" => Self::TxDecode,
			"keypair_generate" => Self::KeypairGenerate,
			"simplicity_info" => Self::SimplicityInfo,
			"simplicity_sighash" => Self::SimplicitySighash,
			"pset_create" => Self::PsetCreate,
			"pset_extract" => Self::PsetExtract,
			"pset_finalize" => Self::PsetFinalize,
			"pset_run" => Self::PsetRun,
			"pset_update_input" => Self::PsetUpdateInput,
			_ => return Err(RpcError::new(ErrorCode::MethodNotFound)),
		};

		Ok(method)
	}
}

/// Default RPC handler that provides basic methods
#[derive(Default)]
pub struct DefaultRpcHandler;

impl RpcHandler for DefaultRpcHandler {
	fn handle(&self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
		let rpc_method = RpcMethod::from_str(method)?;

		match rpc_method {
			RpcMethod::AddressCreate => {
				let req: AddressCreateRequest = parse_params(params)?;
				let result = actions::address::address_create(
					req.pubkey.as_deref(),
					req.script.as_deref(),
					req.blinder.as_deref(),
					req.network.unwrap_or(Network::Liquid),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
			RpcMethod::AddressInspect => {
				let req: AddressInspectRequest = parse_params(params)?;
				let result = actions::address::address_inspect(&req.address).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;

				serialize_result(result)
			}
			RpcMethod::BlockCreate => {
				let req: BlockCreateRequest = parse_params(params)?;

				let block = actions::block::block_create(req.block_info).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;

				let raw_block = hex::encode(elements::encode::serialize(&block));
				serialize_result(BlockCreateResponse {
					raw_block,
				})
			}
			RpcMethod::BlockDecode => {
				let req: BlockDecodeRequest = parse_params(params)?;
				let result = actions::block::block_decode(
					&req.raw_block,
					req.network.unwrap_or(Network::Liquid),
					req.txids.unwrap_or(false),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
			RpcMethod::TxCreate => {
				let req: TxCreateRequest = parse_params(params)?;
				let tx = actions::tx::tx_create(req.tx_info).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;

				let raw_tx = hex::encode(elements::encode::serialize(&tx));
				serialize_result(TxCreateResponse {
					raw_tx,
				})
			}
			RpcMethod::TxDecode => {
				let req: TxDecodeRequest = parse_params(params)?;
				let result =
					actions::tx::tx_decode(&req.raw_tx, req.network.unwrap_or(Network::Liquid))
						.map_err(|e| {
							RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
						})?;

				serialize_result(result)
			}
			RpcMethod::KeypairGenerate => {
				let result = actions::keypair::keypair_generate();

				serialize_result(result)
			}
			RpcMethod::SimplicityInfo => {
				let req: SimplicityInfoRequest = parse_params(params)?;
				let result = actions::simplicity::simplicity_info(
					&req.program,
					req.witness.as_deref(),
					req.state.as_deref(),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
			RpcMethod::SimplicitySighash => {
				let req: SimplicitySighashRequest = parse_params(params)?;
				// TODO(ivanlele): I don't like this flip flop conversion, maybe there is a better API
				let input_utxos = req
					.input_utxos
					.as_ref()
					.map(|v| v.iter().map(String::as_str).collect::<Vec<_>>());

				let result = actions::simplicity::simplicity_sighash(
					&req.tx,
					&req.input_index.to_string(),
					&req.cmr,
					req.control_block.as_deref(),
					req.genesis_hash.as_deref(),
					req.secret_key.as_deref(),
					req.public_key.as_deref(),
					req.signature.as_deref(),
					input_utxos.as_deref(),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;
				serialize_result(result)
			}
			RpcMethod::PsetCreate => {
				let req: PsetCreateRequest = parse_params(params)?;
				let result = actions::simplicity::pset::pset_create(&req.inputs, &req.outputs)
					.map_err(|e| {
						RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
					})?;

				serialize_result(result)
			}
			RpcMethod::PsetExtract => {
				let req: PsetExtractRequest = parse_params(params)?;
				let raw_tx = actions::simplicity::pset::pset_extract(&req.pset).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;

				serialize_result(PsetExtractResponse {
					raw_tx,
				})
			}
			RpcMethod::PsetFinalize => {
				let req: PsetFinalizeRequest = parse_params(params)?;
				let result = actions::simplicity::pset::pset_finalize(
					&req.pset,
					&req.input_index.to_string(),
					&req.program,
					&req.witness,
					req.genesis_hash.as_deref(),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
			RpcMethod::PsetRun => {
				let req: PsetRunRequest = parse_params(params)?;
				let result = actions::simplicity::pset::pset_run(
					&req.pset,
					&req.input_index.to_string(),
					&req.program,
					&req.witness,
					req.genesis_hash.as_deref(),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
			RpcMethod::PsetUpdateInput => {
				let req: PsetUpdateInputRequest = parse_params(params)?;
				let result = actions::simplicity::pset::pset_update_input(
					&req.pset,
					&req.input_index.to_string(),
					&req.input_utxo,
					req.internal_key.as_deref(),
					req.cmr.as_deref(),
					req.state.as_deref(),
				)
				.map_err(|e| RpcError::custom(ErrorCode::InternalError.code(), e.to_string()))?;

				serialize_result(result)
			}
		}
	}
}

impl DefaultRpcHandler {
	fn new() -> Self {
		Self
	}
}

/// Parse parameters from JSON value
fn parse_params<T: serde::de::DeserializeOwned>(params: Option<Value>) -> Result<T, RpcError> {
	let params = params.ok_or_else(|| {
		RpcError::custom(ErrorCode::InvalidParams.code(), "Missing parameters".to_string())
	})?;

	serde_json::from_value(params).map_err(|e| {
		RpcError::custom(ErrorCode::InvalidParams.code(), format!("Invalid parameters: {}", e))
	})
}

/// Serialize result to JSON value
fn serialize_result<T: serde::Serialize>(result: T) -> Result<Value, RpcError> {
	serde_json::to_value(result).map_err(|e| {
		RpcError::custom(
			ErrorCode::InternalError.code(),
			format!("Failed to serialize result: {}", e),
		)
	})
}

/// Create a JSONRPC service with the default handler
pub fn create_service() -> JsonRpcService<DefaultRpcHandler> {
	JsonRpcService::new(DefaultRpcHandler::new())
}
