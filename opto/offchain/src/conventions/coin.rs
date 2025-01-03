#![allow(clippy::tabs_in_doc_comments)]

//! The coin convention is a simple convention that represents a fungible asset
//! on the blockchain. It is a simple object that has a balance, an owner and
//! an asset id. The balance is a u64, the owner is an account id and the asset
//! id is u32. Asset Ids are mapped to AssetIds on the blockchain pallet_assets.
//!
//! the following is an object predicate that matches compatible coin objects:
//!
//! ```rust
//! let coin_pattern = ObjectPattern::default()
//! 	.policies(
//! 		PoliciesPattern::exact()
//! 			.require(opto_stdpred::ids::COIN.with_params(|_: Balance| true))
//! 			.allow(opto_stdpred::ids::NONCE),
//! 	)
//! 	.unlock(opto_stdpred::ids::SR25519)
//! 	.data(|balance: Balance| balance > 0);
//! ```

use {
	crate::{AccountId, AssetId, Balance},
	derive_more::derive::{Deref, From, Into},
	opto_core::{Object, Op, Predicate},
	scale::Decode,
};

#[derive(Debug, Clone, PartialEq)]
pub struct InvalidCoinObject;

#[derive(
	Debug, Clone, PartialEq, PartialOrd, Ord, Hash, Eq, From, Into, Deref, Default,
)]
pub struct CoinBalance(Balance);

impl TryFrom<&Object> for CoinBalance {
	type Error = InvalidCoinObject;

	fn try_from(object: &Object) -> Result<Self, Self::Error> {
		let balance = object
			.policies
			.iter()
			.find_map(|p| {
				if p.id == opto_stdpred::ids::COIN {
					Some(u64::decode(&mut &object.data[..]).ok()?)
				} else {
					None
				}
			})
			.ok_or(InvalidCoinObject)?;

		Ok(Self(balance))
	}
}

/// The ownership of the coin is not following a regonized pattern
/// of SR25519 public key only unlock.
#[derive(Debug, Clone, PartialEq)]
pub struct UnrecognizedCoinOwner;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, From, Into, Deref)]
pub struct CoinOwner(AccountId);

impl core::hash::Hash for CoinOwner {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.0 .0.hash(state);
	}
}

impl TryFrom<&Object> for CoinOwner {
	type Error = UnrecognizedCoinOwner;

	fn try_from(object: &Object) -> Result<Self, Self::Error> {
		let Op::Predicate(Predicate {
			id: opto_stdpred::ids::SR25519,
			params,
		}) = object
			.unlock
			.as_ops()
			.first()
			.ok_or(UnrecognizedCoinOwner)?
		else {
			return Err(UnrecognizedCoinOwner);
		};

		Ok(Self(
			AccountId::decode(&mut &params[..]).map_err(|_| UnrecognizedCoinOwner)?,
		))
	}
}

#[derive(
	Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, From, Into, Deref,
)]
pub struct CoinAsset(AssetId);

impl TryFrom<&Object> for CoinAsset {
	type Error = InvalidCoinObject;

	fn try_from(object: &Object) -> Result<Self, Self::Error> {
		Ok(Self(
			object
				.policies
				.iter()
				.find_map(|p| {
					if p.id == opto_stdpred::ids::COIN {
						Some(AssetId::decode(&mut &object.data[..]).ok()?)
					} else {
						None
					}
				})
				.ok_or(InvalidCoinObject)?,
		))
	}
}
