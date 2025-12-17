use super::PsetError;
use crate::daemon::actions::types::{PsetExtractRequest, PsetExtractResponse};
use elements::encode::serialize_hex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PsetExtractError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("Failed to decode PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("Failed to extract transaction: {0}")]
	TransactionExtract(elements::pset::Error),
}

pub fn extract(req: PsetExtractRequest) -> Result<PsetExtractResponse, PsetExtractError> {
	let pset: elements::pset::PartiallySignedTransaction =
		req.pset.parse().map_err(PsetExtractError::PsetDecode)?;

	let tx = pset.extract_tx().map_err(PsetExtractError::TransactionExtract)?;

	Ok(PsetExtractResponse {
		raw_tx: serialize_hex(&tx),
	})
}
