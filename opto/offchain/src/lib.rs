mod client;
mod model;
mod transition;

pub use {
	client::*, 
	model::api::*, 
	opto_stdpred as stdpred, 
	transition::*, subxt_signer as signer,
	subxt::utils::AccountId32,
};

use {
	core::future::Future,
	opto_core::{
		repr::Compact,
		Digest,
		Expression,
		Object,
		PredicateId,
		Transition,
	},
};

type AssetId = u32;
type Balance = u64;

pub trait ReadOnlyClient {
	type Error;

	/// Retreives the body of an object and its count by its digest.
	fn object(
		&self,
		digest: &Digest,
	) -> impl Future<Output = Result<Option<(Object, u32)>, Self::Error>>;

	/// Retreives predicate's WASM code by its ID.
	fn predicate(
		&self,
		id: PredicateId,
	) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>>;

	/// Balance of an account in a given asset.
	fn asset_balance(
		&self,
		account: &AccountId32,
		asset: AssetId,
	) -> impl Future<Output = Result<Balance, Self::Error>>;

	/// Balance of an account in the native token.
	fn native_balance(
		&self,
		account: &AccountId32,
	) -> impl Future<Output = Result<Balance, Self::Error>>;
}

pub trait MutatingClient {
	type Error;

	/// Wraps an asset into an object
	fn wrap(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		unlock: Option<Expression>,
	) -> impl Future<Output = Result<Object, Self::Error>>;

	/// Unwraps an object into an asset
	fn unwrap(
		&self,
		keypair: &crate::signer::sr25519::Keypair,
		object: &Digest,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Applies a series of state transitions
	/// Returns hashes of objects (Created, Destroyed)
	fn apply(
		&self,
		keypair: &crate::signer::sr25519::Keypair,
		transitions: Vec<Transition<Compact>>,
	) -> impl Future<Output = Result<(Vec<Digest>, Vec<Digest>), Self::Error>>;

	/// Transfer an asset.sssss
	fn asset_transfer(
		&self,
		keypair: &crate::signer::sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		recipient: &AccountId32,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Transfer native tokens.
	fn native_transfer(
		&self,
		keypair: &crate::signer::sr25519::Keypair,
		amount: Balance,
		recipient: &AccountId32,
	) -> impl Future<Output = Result<(), Self::Error>>;
}
