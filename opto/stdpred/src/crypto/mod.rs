pub mod blake2b_256;
pub mod sr25519;

pub use {blake2b_256::blake2b_256, sr25519::sr25519};

pub mod ids {
	pub use super::{
		blake2b_256::blake2b_256_id as blake2b_256,
		sr25519::sr25519_id as sr25519,
	};
}
