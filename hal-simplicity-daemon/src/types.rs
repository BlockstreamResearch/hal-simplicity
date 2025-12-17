use serde::{Deserialize, Serialize};

// Re-exports for proper serialization
pub use elements::bitcoin::secp256k1;
pub use elements::hashes::sha256;
pub use simplicity::bitcoin::secp256k1::schnorr;
pub use simplicity::{Amr, Cmr, Ihr};

// Address types
#[derive(Debug, Serialize, Deserialize)]
pub struct AddressCreateRequest {
	pub network: Option<String>,
	pub pubkey: Option<String>,
	pub script: Option<String>,
	pub blinder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressInspectRequest {
	pub address: String,
}

// Block types
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockCreateRequest {
	pub block_info: String, // JSON string
	pub raw_stdout: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockDecodeRequest {
	pub raw_block: String,
	pub network: Option<String>,
	pub txids: Option<bool>,
}

// Transaction types
#[derive(Debug, Serialize, Deserialize)]
pub struct TxCreateRequest {
	pub tx_info: String, // JSON string
	pub raw_stdout: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxDecodeRequest {
	pub raw_tx: String,
	pub network: Option<String>,
}

// Keypair types
#[derive(Debug, Serialize, Deserialize)]
pub struct KeypairGenerateRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeypairGenerateResponse {
	pub secret: secp256k1::SecretKey,
	pub x_only: secp256k1::XOnlyPublicKey,
	pub parity: secp256k1::Parity,
}

// Simplicity types
#[derive(Debug, Serialize, Deserialize)]
pub struct SimplicityInfoRequest {
	pub program: String,
	pub witness: Option<String>,
	pub state: Option<String>,
	pub network: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplicityInfoResponse {
	pub jets: &'static str,
	pub commit_base64: String,
	pub commit_decode: String,
	pub type_arrow: String,
	pub cmr: Cmr,
	pub liquid_address_unconf: String,
	pub liquid_testnet_address_unconf: String,
	pub is_redeem: bool,
	pub redeem_info: Option<RedeemInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemInfo {
	pub redeem_base64: String,
	pub witness_hex: String,
	pub amr: Amr,
	pub ihr: Ihr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplicitySighashRequest {
	pub tx: String,
	pub input_index: u32,
	pub cmr: String,
	pub control_block: Option<String>,
	pub genesis_hash: Option<String>,
	pub secret_key: Option<String>,
	pub public_key: Option<String>,
	pub signature: Option<String>,
	pub input_utxos: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplicitySighashResponse {
	pub sighash: sha256::Hash,
	pub signature: Option<schnorr::Signature>,
	pub valid_signature: Option<bool>,
}

// PSET types
#[derive(Debug, Serialize, Deserialize)]
pub struct PsetCreateRequest {
	pub inputs: String,  // JSON array string
	pub outputs: String, // JSON array string
	pub network: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetCreateResponse {
	pub pset: String,
	pub updated_values: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetExtractRequest {
	pub pset: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetExtractResponse {
	pub raw_tx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetFinalizeRequest {
	pub pset: String,
	pub input_index: u32,
	pub program: String,
	pub witness: String,
	pub genesis_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetFinalizeResponse {
	pub pset: String,
	pub updated_values: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetRunRequest {
	pub pset: String,
	pub input_index: u32,
	pub program: String,
	pub witness: String,
	pub genesis_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetRunResponse {
	pub success: bool,
	pub jets: Vec<JetCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JetCall {
	pub jet: String,
	pub source_ty: String,
	pub target_ty: String,
	pub success: bool,
	pub input_hex: String,
	pub output_hex: String,
	pub equality_check: Option<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetUpdateInputRequest {
	pub pset: String,
	pub input_index: u32,
	pub input_utxo: String,
	pub internal_key: Option<String>,
	pub cmr: Option<String>,
	pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsetUpdateInputResponse {
	pub pset: String,
	pub updated_values: Vec<String>,
}
