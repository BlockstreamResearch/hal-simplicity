use crate::jsonrpc::{ErrorCode, JsonRpcService, RpcError, RpcHandler};
use serde_json::Value;

/// Default RPC handler that provides basic methods
pub struct DefaultRpcHandler;

impl RpcHandler for DefaultRpcHandler {
	fn handle(&self, method: &str, _params: Option<Value>) -> Result<Value, RpcError> {
		match method {
			_ => Err(RpcError::new(ErrorCode::MethodNotFound)),
		}
	}
}

impl DefaultRpcHandler {
	pub fn new() -> Self {
		Self
	}
}

/// Create a JSONRPC service with the default handler
pub fn create_service() -> JsonRpcService<DefaultRpcHandler> {
	JsonRpcService::new(DefaultRpcHandler::new())
}
