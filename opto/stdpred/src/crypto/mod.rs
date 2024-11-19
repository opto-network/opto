pub mod blake2b_256;
pub mod ed25519;
pub mod groth16_bn254;
pub mod signature;
pub mod sp1_groth16;
pub mod sr25519;

pub use {
	blake2b_256::blake2b_256,
	ed25519::ed25519,
	groth16_bn254::groth16_bn254,
	sp1_groth16::sp1_groth16,
	sr25519::sr25519,
};

pub mod ids {
	pub use super::{
		blake2b_256::blake2b_256_id as blake2b_256,
		ed25519::ed25519_id as ed25519,
		groth16_bn254::groth16_bn254_id as groth16_bn254,
		sp1_groth16::sp1_groth16_id as sp1_groth16,
		sr25519::sr25519_id as sr25519,
	};
}
