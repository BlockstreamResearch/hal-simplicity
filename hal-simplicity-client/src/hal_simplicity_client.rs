use hal_simplicity_daemon::jsonrpc::{RpcError, RpcRequest, RpcResponse};
use hal_simplicity_daemon::types::*;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

/// JSON-RPC client errors
#[derive(Debug, Error)]
pub enum ClientError {
	#[error("HTTP request failed: {0}")]
	Http(#[from] reqwest::Error),

	#[error("RPC error: {0}")]
	Rpc(#[from] RpcError),

	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	#[error("Invalid response: {0}")]
	InvalidResponse(String),

	#[error("Connection refused: daemon not running at {0}")]
	ConnectionRefused(String),
}

/// HAL Simplicity client for hal-simplicity-daemon
pub struct HalSimplicity {
	client: Client,
	url: String,
	next_id: std::sync::atomic::AtomicU64,
}

impl HalSimplicity {
	/// Create a new JSON-RPC client
	pub fn new(url: String) -> Result<Self, ClientError> {
		let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

		Ok(Self {
			client,
			url,
			next_id: std::sync::atomic::AtomicU64::new(1),
		})
	}

	/// Create a client with default URL (http://localhost:28579)
	pub fn default() -> Result<Self, ClientError> {
		Self::new("http://localhost:28579".to_string())
	}

	/// Get next request ID
	fn next_id(&self) -> u64 {
		self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
	}

	/// Send a JSON-RPC request and return the response
	fn call<T: DeserializeOwned>(
		&self,
		method: &str,
		params: Option<Value>,
	) -> Result<T, ClientError> {
		let id = self.next_id();
		let request = RpcRequest::new(method.to_string(), params, Some(Value::from(id)));

		let json_request = serde_json::to_string(&request)?;

		let response = self
			.client
			.post(&self.url)
			.header("Content-Type", "application/json")
			.body(json_request)
			.send()
			.map_err(|e| {
				if e.is_connect() {
					ClientError::ConnectionRefused(self.url.clone())
				} else {
					ClientError::Http(e)
				}
			})?;

		let status = response.status();
		let body = response.text()?;

		if !status.is_success() {
			return Err(ClientError::InvalidResponse(format!("HTTP {}: {}", status, body)));
		}

		let rpc_response: RpcResponse = serde_json::from_str(&body)?;

		if let Some(error) = rpc_response.error {
			return Err(ClientError::Rpc(error));
		}

		let result = rpc_response.result.ok_or_else(|| {
			ClientError::InvalidResponse("Response missing both result and error".to_string())
		})?;

		Ok(serde_json::from_value(result)?)
	}

	/// Check if the daemon is reachable
	pub fn ping(&self) -> Result<(), ClientError> {
		// Try to generate a keypair as a ping (lightweight operation)
		// We could use any RPC method, but keypair_generate is simple
		let _result = self.keypair_generate()?;
		Ok(())
	}

	/// Get the daemon URL
	pub fn url(&self) -> &str {
		&self.url
	}

	/// Set a custom timeout for requests
	pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, ClientError> {
		self.client = Client::builder().timeout(timeout).build()?;
		Ok(self)
	}

	// Address methods

	/// Create an Elements address
	pub fn address_create(
		&self,
		network: Option<String>,
		pubkey: Option<String>,
		script: Option<String>,
		blinder: Option<String>,
	) -> Result<Value, ClientError> {
		let params = AddressCreateRequest {
			network,
			pubkey,
			script,
			blinder,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("address_create", Some(params_value))
	}

	/// Inspect an Elements address
	pub fn address_inspect(&self, address: String) -> Result<Value, ClientError> {
		let params = AddressInspectRequest {
			address,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("address_inspect", Some(params_value))
	}

	// Block methods

	/// Create a block from JSON data
	pub fn block_create(
		&self,
		block_info: String,
		raw_stdout: Option<bool>,
	) -> Result<Value, ClientError> {
		let params = BlockCreateRequest {
			block_info,
			raw_stdout,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("block_create", Some(params_value))
	}

	/// Decode a raw block
	pub fn block_decode(
		&self,
		raw_block: String,
		network: Option<String>,
		txids: Option<bool>,
	) -> Result<Value, ClientError> {
		let params = BlockDecodeRequest {
			raw_block,
			network,
			txids,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("block_decode", Some(params_value))
	}

	// Transaction methods

	/// Create a transaction from JSON data
	pub fn tx_create(
		&self,
		tx_info: String,
		raw_stdout: Option<bool>,
	) -> Result<Value, ClientError> {
		let params = TxCreateRequest {
			tx_info,
			raw_stdout,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("tx_create", Some(params_value))
	}

	/// Decode a raw transaction
	pub fn tx_decode(&self, raw_tx: String, network: Option<String>) -> Result<Value, ClientError> {
		let params = TxDecodeRequest {
			raw_tx,
			network,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("tx_decode", Some(params_value))
	}

	// Keypair methods

	/// Generate a new keypair
	pub fn keypair_generate(&self) -> Result<KeypairGenerateResponse, ClientError> {
		let params = KeypairGenerateRequest {};
		let params_value = serde_json::to_value(params)?;
		self.call("keypair_generate", Some(params_value))
	}

	// Simplicity methods

	/// Get information about a Simplicity program
	pub fn simplicity_info(
		&self,
		program: String,
		witness: Option<String>,
		state: Option<String>,
		network: Option<String>,
	) -> Result<Value, ClientError> {
		let params = SimplicityInfoRequest {
			program,
			witness,
			state,
			network,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("simplicity_info", Some(params_value))
	}

	/// Compute and optionally sign a Simplicity sighash
	pub fn simplicity_sighash(
		&self,
		tx: String,
		input_index: u32,
		cmr: String,
		control_block: Option<String>,
		genesis_hash: Option<String>,
		secret_key: Option<String>,
		public_key: Option<String>,
		signature: Option<String>,
		input_utxos: Option<Vec<String>>,
	) -> Result<SimplicitySighashResponse, ClientError> {
		let params = SimplicitySighashRequest {
			tx,
			input_index,
			cmr,
			control_block,
			genesis_hash,
			secret_key,
			public_key,
			signature,
			input_utxos,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("simplicity_sighash", Some(params_value))
	}

	// PSET methods

	/// Create a new PSET
	pub fn pset_create(
		&self,
		inputs: String,
		outputs: String,
		network: Option<String>,
	) -> Result<PsetCreateResponse, ClientError> {
		let params = PsetCreateRequest {
			inputs,
			outputs,
			network,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("pset_create", Some(params_value))
	}

	/// Extract a transaction from a PSET
	pub fn pset_extract(&self, pset: String) -> Result<PsetExtractResponse, ClientError> {
		let params = PsetExtractRequest {
			pset,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("pset_extract", Some(params_value))
	}

	/// Finalize a PSET input with a Simplicity program
	pub fn pset_finalize(
		&self,
		pset: String,
		input_index: u32,
		program: String,
		witness: String,
		genesis_hash: Option<String>,
	) -> Result<PsetFinalizeResponse, ClientError> {
		let params = PsetFinalizeRequest {
			pset,
			input_index,
			program,
			witness,
			genesis_hash,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("pset_finalize", Some(params_value))
	}

	/// Run a Simplicity program in a PSET context
	pub fn pset_run(
		&self,
		pset: String,
		input_index: u32,
		program: String,
		witness: String,
		genesis_hash: Option<String>,
	) -> Result<PsetRunResponse, ClientError> {
		let params = PsetRunRequest {
			pset,
			input_index,
			program,
			witness,
			genesis_hash,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("pset_run", Some(params_value))
	}

	/// Update a PSET input with additional information
	pub fn pset_update_input(
		&self,
		pset: String,
		input_index: u32,
		input_utxo: String,
		internal_key: Option<String>,
		cmr: Option<String>,
		state: Option<String>,
	) -> Result<PsetUpdateInputResponse, ClientError> {
		let params = PsetUpdateInputRequest {
			pset,
			input_index,
			input_utxo,
			internal_key,
			cmr,
			state,
		};
		let params_value = serde_json::to_value(params)?;
		self.call("pset_update_input", Some(params_value))
	}
}
