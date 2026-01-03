// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use elements::encode::serialize_hex;

use super::PsetError;

#[derive(Debug, thiserror::Error)]
pub enum PsetExtractError {
	#[error(transparent)]
	SharedError(#[from] PsetError),

	#[error("invalid PSET: {0}")]
	PsetDecode(elements::pset::ParseError),

	#[error("failed to extract transaction: {0}")]
	TransactionExtract(elements::pset::Error),
}

/// Extract a raw transaction from a completed PSET
pub fn pset_extract(pset_b64: &str) -> Result<String, PsetExtractError> {
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetExtractError::PsetDecode)?;

	let tx = pset.extract_tx().map_err(PsetExtractError::TransactionExtract)?;
	Ok(serialize_hex(&tx))
}
