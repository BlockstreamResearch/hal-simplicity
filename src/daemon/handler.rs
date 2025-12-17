use crate::jsonrpc::{ErrorCode, JsonRpcService, RpcError, RpcHandler};
use serde_json::Value;

use super::actions::{self, types::*};

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

impl RpcMethod {
	pub fn from_str(s: &str) -> Option<Self> {
		match s {
			"address_create" => Some(Self::AddressCreate),
			"address_inspect" => Some(Self::AddressInspect),
			"block_create" => Some(Self::BlockCreate),
			"block_decode" => Some(Self::BlockDecode),
			"tx_create" => Some(Self::TxCreate),
			"tx_decode" => Some(Self::TxDecode),
			"keypair_generate" => Some(Self::KeypairGenerate),
			"simplicity_info" => Some(Self::SimplicityInfo),
			"simplicity_sighash" => Some(Self::SimplicitySighash),
			"pset_create" => Some(Self::PsetCreate),
			"pset_extract" => Some(Self::PsetExtract),
			"pset_finalize" => Some(Self::PsetFinalize),
			"pset_run" => Some(Self::PsetRun),
			"pset_update_input" => Some(Self::PsetUpdateInput),
			_ => None,
		}
	}

	#[allow(dead_code)]
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::AddressCreate => "address_create",
			Self::AddressInspect => "address_inspect",
			Self::BlockCreate => "block_create",
			Self::BlockDecode => "block_decode",
			Self::TxCreate => "tx_create",
			Self::TxDecode => "tx_decode",
			Self::KeypairGenerate => "keypair_generate",
			Self::SimplicityInfo => "simplicity_info",
			Self::SimplicitySighash => "simplicity_sighash",
			Self::PsetCreate => "pset_create",
			Self::PsetExtract => "pset_extract",
			Self::PsetFinalize => "pset_finalize",
			Self::PsetRun => "pset_run",
			Self::PsetUpdateInput => "pset_update_input",
		}
	}
}

/// Default RPC handler that provides basic methods
pub struct DefaultRpcHandler;

impl RpcHandler for DefaultRpcHandler {
	fn handle(&self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
		let rpc_method =
			RpcMethod::from_str(method).ok_or_else(|| RpcError::new(ErrorCode::MethodNotFound))?;

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
	pub fn new() -> Self {
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
