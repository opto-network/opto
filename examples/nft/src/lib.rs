#[cfg(feature = "offchain")]
mod api;

#[cfg(feature = "offchain")]
pub use api::*;

#[cfg(feature = "onchain")]
mod policies;

#[cfg(feature = "onchain")]
pub use policies::*;

/// A serialized version of this struct is the contents of the `nft` policy.
/// The uniqueness of an nft is the hash of this struct.
#[derive(Debug, scale::Encode, scale::Decode, scale::MaxEncodedLen)]
pub struct NftIdentity {
	pub mint: opto::Digest,
	pub tag: opto::Digest,
	pub mutable: bool,
}
impl scale::ConstEncodedLen for NftIdentity {}
