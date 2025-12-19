use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Deserialize HexBytes from both borrowed and owned strings.
/// This is needed because hal's HexBytes::deserialize only accepts &str,
/// but when deserializing from JSON (e.g., via serde_json), the string is owned.
pub mod hex_bytes {
	use super::*;

	pub fn serialize<S>(value: &Option<::hal::HexBytes>, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match value {
			Some(hex_bytes) => hex_bytes.serialize(s),
			None => s.serialize_none(),
		}
	}

	pub fn deserialize<'de, D>(d: D) -> Result<Option<::hal::HexBytes>, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::Error;

		Option::<String>::deserialize(d)?
			.map(|hex_str| {
				hex::decode(&hex_str).map(::hal::HexBytes::from).map_err(D::Error::custom)
			})
			.transpose()
	}
}
