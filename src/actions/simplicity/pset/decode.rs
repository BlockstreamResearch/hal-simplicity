// Copyright 2025 Andrew Poelstra
// SPDX-License-Identifier: CC0-1.0

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum PsetDecodeError {
	#[error("invalid PSET: {0}")]
	PsetParse(elements::pset::ParseError),

	#[error("failed to serialize PSET to JSON: {0}")]
	JsonSerialize(serde_json::Error),
}

/// Decode a PSET into a JSON representation
pub fn pset_decode(pset_b64: &str) -> Result<Value, PsetDecodeError> {
	let pset: elements::pset::PartiallySignedTransaction =
		pset_b64.parse().map_err(PsetDecodeError::PsetParse)?;
	serde_json::to_value(&pset).map_err(PsetDecodeError::JsonSerialize)
}
