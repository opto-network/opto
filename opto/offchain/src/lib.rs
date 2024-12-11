use {
	core::future::Future,
	futures::Stream,
	opto_core::{
		repr::Compact,
		Digest,
		Expression,
		Object,
		PredicateId,
		Transition,
	},
};

mod client;
mod ext;
mod transition;

mod model {
	include!(concat!(env!("OUT_DIR"), "/model.rs"));
}

pub use {
	client::*,
	ext::*,
	futures,
	model::api::*,
	opto_stdpred as stdpred,
	subxt::utils::AccountId32,
	subxt_signer as signer,
	transition::*,
};

type AssetId = u32;
type Balance = u64;

pub trait StreamingClient {
	type Error;

	/// Returns a stream of state transitions.
	fn transitions(
		&self,
	) -> impl Stream<Item = Result<Transition<Compact>, Self::Error>>;
}

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
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Installs a new predicate or a group or predicates.
	/// This function accepts either a WASAM binary or a CAR file.
	fn install(
		&self,
		keypair: &crate::signer::sr25519::Keypair,
		wasm_or_car: Vec<u8>,
	) -> impl Future<Output = Result<(), Self::Error>>;

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
