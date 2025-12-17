use elements::bitcoin::secp256k1::{self, rand};

use super::types::KeypairGenerateResponse;

pub fn generate() -> KeypairGenerateResponse {
	let (secret, public) = secp256k1::generate_keypair(&mut rand::thread_rng());
	let (x_only, parity) = public.x_only_public_key();

	KeypairGenerateResponse {
		secret,
		x_only,
		parity,
	}
}
