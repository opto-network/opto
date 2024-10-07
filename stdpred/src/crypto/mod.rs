pub mod ed25519;
pub mod signature;
pub mod sr25519;

pub use {ed25519::ed25519, sr25519::sr25519};

pub mod ids {
	pub use super::{
		ed25519::ed25519_id as ed25519,
		sr25519::sr25519_id as sr25519,
	};
}
