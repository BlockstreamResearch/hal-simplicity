use elements::confidential::{Asset, Nonce, Value};
use elements::AssetId;
use serde::{Deserialize, Serialize};

use crate::{GetInfo, HexBytes, Network};

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidentialType {
	Null,
	Explicit,
	Confidential,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ConfidentialValueInfo {
	#[serde(rename = "type")]
	pub type_: ConfidentialType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub value: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub commitment: Option<HexBytes>,
}

impl GetInfo<ConfidentialValueInfo> for Value {
	fn get_info(&self, _network: Network) -> ConfidentialValueInfo {
		ConfidentialValueInfo {
			type_: match self {
				Value::Null => ConfidentialType::Null,
				Value::Explicit(..) => ConfidentialType::Explicit,
				Value::Confidential(..) => ConfidentialType::Confidential,
			},
			value: match self {
				Value::Explicit(v) => Some(*v),
				_ => None,
			},
			commitment: match self {
				Value::Confidential(pk) => Some(pk.serialize()[..].into()),
				_ => None,
			},
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidentialAssetLabel {
	LiquidBitcoin,
}

impl ConfidentialAssetLabel {
	pub fn from_asset_id(id: AssetId) -> Option<ConfidentialAssetLabel> {
		match id.to_string().as_str() {
			"6f0279e9ed041c3d710a9f57d0c02928416460c4b722ae3457a11eec381c526d" => {
				Some(ConfidentialAssetLabel::LiquidBitcoin)
			}
			_ => None,
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ConfidentialAssetInfo {
	#[serde(rename = "type")]
	pub type_: ConfidentialType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub asset: Option<AssetId>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub commitment: Option<HexBytes>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub label: Option<ConfidentialAssetLabel>,
}

impl GetInfo<ConfidentialAssetInfo> for Asset {
	fn get_info(&self, _network: Network) -> ConfidentialAssetInfo {
		ConfidentialAssetInfo {
			type_: match self {
				Asset::Null => ConfidentialType::Null,
				Asset::Explicit(..) => ConfidentialType::Explicit,
				Asset::Confidential(..) => ConfidentialType::Confidential,
			},
			asset: match self {
				Asset::Explicit(a) => Some(*a),
				_ => None,
			},
			commitment: match self {
				Asset::Confidential(pk) => Some(pk.serialize()[..].into()),
				_ => None,
			},
			label: match self {
				Asset::Explicit(a) => ConfidentialAssetLabel::from_asset_id(*a),
				_ => None,
			},
		}
	}
}

impl GetInfo<ConfidentialAssetInfo> for AssetId {
	fn get_info(&self, _network: Network) -> ConfidentialAssetInfo {
		ConfidentialAssetInfo {
			type_: ConfidentialType::Explicit,
			asset: Some(*self),
			commitment: None,
			label: ConfidentialAssetLabel::from_asset_id(*self),
		}
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ConfidentialNonceInfo {
	#[serde(rename = "type")]
	pub type_: ConfidentialType,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nonce: Option<HexBytes>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub commitment: Option<HexBytes>,
}

impl GetInfo<ConfidentialNonceInfo> for Nonce {
	fn get_info(&self, _network: Network) -> ConfidentialNonceInfo {
		ConfidentialNonceInfo {
			type_: match self {
				Nonce::Null => ConfidentialType::Null,
				Nonce::Explicit(..) => ConfidentialType::Explicit,
				Nonce::Confidential(..) => ConfidentialType::Confidential,
			},
			nonce: match self {
				Nonce::Explicit(n) => Some(n[..].into()),
				_ => None,
			},
			commitment: match self {
				Nonce::Confidential(pk) => Some(pk.serialize()[..].into()),
				_ => None,
			},
		}
	}
}
