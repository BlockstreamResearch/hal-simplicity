//! Simple JSONRPC 2.0 implementation
//!
//! <https://www.jsonrpc.org/specification>

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// JSONRPC 2.0 Error codes as defined in the specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
	ParseError = -32700,
	InvalidRequest = -32600,
	MethodNotFound = -32601,
	InvalidParams = -32602,
	InternalError = -32603,
}

impl ErrorCode {
	pub fn code(&self) -> i64 {
		*self as i64
	}

	pub fn message(&self) -> &str {
		match self {
			ErrorCode::ParseError => "Parse error",
			ErrorCode::InvalidRequest => "Invalid Request",
			ErrorCode::MethodNotFound => "Method not found",
			ErrorCode::InvalidParams => "Invalid params",
			ErrorCode::InternalError => "Internal error",
		}
	}
}

/// JSONRPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
	pub code: i64,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<Value>,
}

impl RpcError {
	pub fn new(error_code: ErrorCode) -> Self {
		Self {
			code: error_code.code(),
			message: error_code.message().to_string(),
			data: None,
		}
	}

	pub fn with_data(mut self, data: Value) -> Self {
		self.data = Some(data);
		self
	}

	pub fn custom(code: i64, message: String) -> Self {
		Self {
			code,
			message,
			data: None,
		}
	}
}

impl fmt::Display for RpcError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "RPC Error {}: {}", self.code, self.message)
	}
}

impl std::error::Error for RpcError {}

/// JSONRPC 2.0 Request object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
	pub jsonrpc: String,
	pub method: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub params: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<Value>,
}

impl RpcRequest {
	pub fn new(method: String, params: Option<Value>, id: Option<Value>) -> Self {
		Self {
			jsonrpc: "2.0".to_string(),
			method,
			params,
			id,
		}
	}

	/// Check if this is a notification (no id field)
	pub fn is_notification(&self) -> bool {
		self.id.is_none()
	}

	/// Validate the request according to JSONRPC 2.0 spec
	pub fn validate(&self) -> Result<(), RpcError> {
		if self.jsonrpc != "2.0" {
			return Err(RpcError::new(ErrorCode::InvalidRequest)
				.with_data(Value::String("jsonrpc field must be '2.0'".to_string())));
		}

		if self.method.is_empty() {
			return Err(RpcError::new(ErrorCode::InvalidRequest)
				.with_data(Value::String("method field cannot be empty".to_string())));
		}

		Ok(())
	}
}

/// JSONRPC 2.0 Response object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
	pub jsonrpc: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub result: Option<Value>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<RpcError>,
	pub id: Value,
}

impl RpcResponse {
	pub fn success(result: Value, id: Value) -> Self {
		Self {
			jsonrpc: "2.0".to_string(),
			result: Some(result),
			error: None,
			id,
		}
	}

	pub fn error(error: RpcError, id: Value) -> Self {
		Self {
			jsonrpc: "2.0".to_string(),
			result: None,
			error: Some(error),
			id,
		}
	}
}

/// Represents either a single request or batch requests
#[derive(Debug)]
pub enum RpcCall {
	Single(RpcRequest),
	Batch(Vec<RpcRequest>),
}

impl RpcCall {
	/// Parse a JSON string into an RPC call
	pub fn from_json(json: &str) -> Result<Self, RpcError> {
		// Try parsing as a single request first
		if let Ok(request) = serde_json::from_str::<RpcRequest>(json) {
			request.validate()?;
			return Ok(RpcCall::Single(request));
		}

		// Try psrsing as a batch request
		match serde_json::from_str::<Vec<RpcRequest>>(json) {
			Ok(requests) => {
				if requests.is_empty() {
					return Err(RpcError::new(ErrorCode::InvalidRequest)
						.with_data(Value::String("batch request cannot be empty".to_string())));
				}

				for request in &requests {
					request.validate()?;
				}

				Ok(RpcCall::Batch(requests))
			}
			Err(_) => Err(RpcError::new(ErrorCode::ParseError)),
		}
	}
}

/// Represents either a single response or batch responses
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum RpcOutput {
	Single(RpcResponse),
	Batch(Vec<RpcResponse>),
}

impl RpcOutput {
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}
}

/// Handler trait for RPC methods
pub trait RpcHandler: Send + Sync {
	fn handle(&self, method: &str, params: Option<Value>) -> Result<Value, RpcError>;
}

/// Main JSONRPC service
pub struct JsonRpcService<H: RpcHandler> {
	handler: H,
}

impl<H: RpcHandler> JsonRpcService<H> {
	pub fn new(handler: H) -> Self {
		Self {
			handler,
		}
	}

	/// Process a raw JSON string and return a JSON response
	pub fn handle_raw(&self, json: &str) -> String {
		match RpcCall::from_json(json) {
			Ok(call) => match call {
				RpcCall::Single(request) => {
					let response = self.handle_single(request);
					if let Some(resp) = response {
						serde_json::to_string(&resp).unwrap_or_else(|_| {
							serde_json::to_string(&RpcResponse::error(
								RpcError::new(ErrorCode::InternalError),
								Value::Null,
							))
							.unwrap()
						})
					} else {
						// Notification - no response
						String::new()
					}
				}
				RpcCall::Batch(requests) => {
					let responses = self.handle_batch(requests);
					if responses.is_empty() {
						// All notifications - no response
						String::new()
					} else {
						RpcOutput::Batch(responses).to_json().unwrap_or_else(|_| {
							serde_json::to_string(&RpcResponse::error(
								RpcError::new(ErrorCode::InternalError),
								Value::Null,
							))
							.unwrap()
						})
					}
				}
			},
			Err(error) => {
				serde_json::to_string(&RpcResponse::error(error, Value::Null)).expect("should ")
			}
		}
	}

	/// Handle a single RPC request
	fn handle_single(&self, request: RpcRequest) -> Option<RpcResponse> {
		// Notifications don't get responses
		if request.is_notification() {
			let _ = self.handler.handle(&request.method, request.params);
			return None;
		}

		let id = request.id.clone().unwrap_or(Value::Null);

		let response = match self.handler.handle(&request.method, request.params) {
			Ok(result) => RpcResponse::success(result, id),
			Err(error) => RpcResponse::error(error, id),
		};

		Some(response)
	}

	/// Handle a batch of RPC requests
	fn handle_batch(&self, requests: Vec<RpcRequest>) -> Vec<RpcResponse> {
		requests.into_iter().filter_map(|request| self.handle_single(request)).collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct TestHandler;

	impl RpcHandler for TestHandler {
		fn handle(&self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
			match method {
				"echo" => Ok(params.unwrap_or(Value::Null)),
				"add" => {
					let params = params.ok_or_else(|| RpcError::new(ErrorCode::InvalidParams))?;
					let array =
						params.as_array().ok_or_else(|| RpcError::new(ErrorCode::InvalidParams))?;
					if array.len() != 2 {
						return Err(RpcError::new(ErrorCode::InvalidParams));
					}
					let a =
						array[0].as_i64().ok_or_else(|| RpcError::new(ErrorCode::InvalidParams))?;
					let b =
						array[1].as_i64().ok_or_else(|| RpcError::new(ErrorCode::InvalidParams))?;
					Ok(Value::Number((a + b).into()))
				}
				_ => Err(RpcError::new(ErrorCode::MethodNotFound)),
			}
		}
	}

	#[test]
	fn test_single_request() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"{"jsonrpc":"2.0","method":"echo","params":"hello","id":1}"#;
		let response = service.handle_raw(request);
		assert!(response.contains(r#""result":"hello""#));
		assert!(response.contains(r#""id":1"#));
	}

	#[test]
	fn test_notification() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"{"jsonrpc":"2.0","method":"echo","params":"hello"}"#;
		let response = service.handle_raw(request);
		assert_eq!(response, "");
	}

	#[test]
	fn test_batch_request() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"[
            {"jsonrpc":"2.0","method":"add","params":[1,2],"id":1},
            {"jsonrpc":"2.0","method":"add","params":[3,4],"id":2}
        ]"#;
		let response = service.handle_raw(request);
		assert!(response.contains(r#""result":3"#));
		assert!(response.contains(r#""result":7"#));
	}

	#[test]
	fn test_method_not_found() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"{"jsonrpc":"2.0","method":"unknown","id":1}"#;
		let response = service.handle_raw(request);
		assert!(response.contains(r#""code":-32601"#));
		assert!(response.contains("Method not found"));
	}

	#[test]
	fn test_invalid_json() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"{"jsonrpc":"2.0","method":"#;
		let response = service.handle_raw(request);
		assert!(response.contains(r#""code":-32700"#));
	}

	#[test]
	fn test_invalid_request() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"{"jsonrpc":"1.0","method":"echo","id":1}"#;
		let response = service.handle_raw(request);
		assert!(response.contains(r#""code":-32600"#));
	}

	#[test]
	fn test_batch_with_notifications() {
		let service = JsonRpcService::new(TestHandler);
		let request = r#"[
            {"jsonrpc":"2.0","method":"echo","params":"notify"},
            {"jsonrpc":"2.0","method":"add","params":[1,2],"id":1}
        ]"#;
		let response = service.handle_raw(request);
		// Should only have one response (the non-notification)
		assert!(response.contains(r#""result":3"#));
		assert!(response.contains(r#""id":1"#));
	}
}
