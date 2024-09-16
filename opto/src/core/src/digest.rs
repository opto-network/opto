pub use blake2::Digest as DigestBuilder;
use {
	core::{
		fmt::{Debug, Display},
		hash::Hash,
		ops::Deref,
	},
	derive_more::derive::From,
	scale::{Decode, Encode, MaxEncodedLen},
	scale_info::TypeInfo,
	serde::{Deserialize, Serialize},
};

pub type DefaultOutputSize = blake2::digest::consts::U32;
pub type Hasher<Size = DefaultOutputSize> = blake2::Blake2b<Size>;

/// This is the canonical hash function used across the entire system.
/// It is a 256-bit BLAKE2b hash.
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	TypeInfo,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	From,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct Digest([u8; 32]);

pub trait Hashable: Encode {
	fn digest(&self) -> Digest {
		Digest::compute(&self.encode())
	}
}

impl<T: Encode> Hashable for T {}

impl MaxEncodedLen for Digest {
	fn max_encoded_len() -> usize {
		core::mem::size_of::<Self>()
	}
}

impl From<blake2::digest::Output<Hasher>> for Digest {
	fn from(output: blake2::digest::Output<Hasher>) -> Self {
		Digest(output.into())
	}
}

impl Digest {
	// 0xb220 is the code for blake2b-256
	// full table is here;
	// https://github.com/multiformats/multicodec/blob/master/table.csv
	pub const MULTIHASH_CODE: u64 = 0xb220;
	pub const SIZE: usize = size_of::<Self>();

	pub fn compute(data: &[u8]) -> Self {
		use blake2::Digest as _;
		let mut hasher = Self::hasher();
		hasher.update(data);
		Self(hasher.finalize().into())
	}

	/// Get an instance of the default hasher
	pub fn hasher() -> Hasher {
		<Hasher as blake2::Digest>::new()
	}
}

impl Deref for Digest {
	type Target = [u8; 32];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AsRef<[u8]> for Digest {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl Debug for Digest {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "0x{}", hex::encode(self.0))
	}
}

impl Display for Digest {
	/// Display only first 4 bytes and last bytes of the hash
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"0x{}..{}",
			hex::encode(&self.0[..4]),
			hex::encode(&self.0[28..])
		)
	}
}

#[cfg(test)]
mod test {
	use super::Digest;

	impl Digest {
		pub const fn zero() -> Self {
			Digest([0; 32])
		}
	}

	impl From<&str> for Digest {
		fn from(value: &str) -> Self {
			Self::compute(value.as_bytes())
		}
	}

	impl From<&[u8]> for Digest {
		fn from(value: &[u8]) -> Self {
			Self::compute(value)
		}
	}
}
