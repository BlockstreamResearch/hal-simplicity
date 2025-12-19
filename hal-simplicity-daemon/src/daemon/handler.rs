use std::str::FromStr;

use crate::jsonrpc::{ErrorCode, JsonRpcService, RpcError, RpcHandler};
use serde_json::Value;

use super::actions;
use crate::types::*;

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
				let result = actions::address::create(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::AddressInspect => {
				let req: AddressInspectRequest = parse_params(params)?;
				let result = actions::address::inspect(&req.address).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::BlockCreate => {
				let req: BlockCreateRequest = parse_params(params)?;
				let result = actions::block::create(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::BlockDecode => {
				let req: BlockDecodeRequest = parse_params(params)?;
				let result = actions::block::decode(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				Ok(result)
			}
			RpcMethod::TxCreate => {
				let req: TxCreateRequest = parse_params(params)?;
				let result = actions::tx::create(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::TxDecode => {
				let req: TxDecodeRequest = parse_params(params)?;
				let result = actions::tx::decode(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				Ok(result)
			}
			RpcMethod::KeypairGenerate => {
				let result = actions::keypair::generate();
				serialize_result(result)
			}
			RpcMethod::SimplicityInfo => {
				let req: SimplicityInfoRequest = parse_params(params)?;
				let result = actions::simplicity::info(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::SimplicitySighash => {
				let req: SimplicitySighashRequest = parse_params(params)?;
				let result = actions::simplicity::sighash(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::PsetCreate => {
				let req: PsetCreateRequest = parse_params(params)?;
				let result = actions::simplicity::create(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::PsetExtract => {
				let req: PsetExtractRequest = parse_params(params)?;
				let result = actions::simplicity::extract(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::PsetFinalize => {
				let req: PsetFinalizeRequest = parse_params(params)?;
				let result = actions::simplicity::finalize(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::PsetRun => {
				let req: PsetRunRequest = parse_params(params)?;
				let result = actions::simplicity::run(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
				serialize_result(result)
			}
			RpcMethod::PsetUpdateInput => {
				let req: PsetUpdateInputRequest = parse_params(params)?;
				let result = actions::simplicity::update_input(req).map_err(|e| {
					RpcError::custom(ErrorCode::InternalError.code(), e.to_string())
				})?;
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
