use elements::bitcoin::secp256k1::{self, rand};

#[derive(serde::Serialize)]
pub struct KeypairInfo {
	pub secret: secp256k1::SecretKey,
	pub x_only: secp256k1::XOnlyPublicKey,
	pub parity: secp256k1::Parity,
}

/// Generate a random keypair.
pub fn keypair_generate() -> KeypairInfo {
	let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
	let (x_only, parity) = public.x_only_public_key();

	KeypairInfo {
		secret,
		x_only,
		parity,
	}
}
