pub mod blake2b_256;
pub mod ed25519;
pub mod signature;
pub mod sr25519;

pub use {blake2b_256::blake2b_256, ed25519::ed25519, sr25519::sr25519};

pub mod ids {
	pub use super::{
		blake2b_256::blake2b_256_id as blake2b_256,
		ed25519::ed25519_id as ed25519,
		sr25519::sr25519_id as sr25519,
	};
}
