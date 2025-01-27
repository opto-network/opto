use {
	crate::signer::sr25519,
	core::future::Future,
	futures::Stream,
	model::{
		model::{
			objects::storage::types::timestamp::Timestamp,
			system::events::extrinsic_failed::DispatchError,
		},
		objects::{model::ActiveObject, pallet::Event},
	},
	opto_core::*,
	scale::{Decode, Encode},
	std::sync::Arc,
	subxt::{
		storage::Storage,
		tx::TxClient,
		utils::AccountId32,
		Config,
		OnlineClient,
		SubstrateConfig,
	},
	tokio_stream::StreamExt,
};

pub mod model;
mod mutable;
mod read;
mod stream;

pub use futures;

type AssetId = u32;
type Balance = u64;

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct Reservation {
	pub object: Digest,
	pub until: Timestamp,
	pub by: AccountId32,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct Release {
	pub object: Digest,
	pub by: AccountId32,
	pub consumed: bool,
}

pub trait StreamingClient {
	type Error;

	/// Returns a stream of state transitions.
	fn transitions(
		&self,
	) -> impl Stream<Item = Result<Transition<Compact>, Self::Error>> {
		self.events().filter_map(|event| match event {
			Ok(Event::StateTransitioned { transition }) => Some(Ok(transition)),
			_ => None,
		})
	}

	/// Returns a stream of notifications about object reservations
	fn reservations(
		&self,
	) -> impl Stream<Item = Result<Reservation, Self::Error>> {
		self.events().filter_map(|event| match event {
			Ok(Event::ObjectReserved { object, by, until }) => {
				Some(Ok(Reservation { object, by, until }))
			}
			_ => None,
		})
	}

	/// Returns a stream of notifications about object reservation releases
	fn releases(&self) -> impl Stream<Item = Result<Release, Self::Error>> {
		self.events().filter_map(|event| match event {
			Ok(Event::ReservationReleased {
				object,
				by,
				consumed,
			}) => Some(Ok(Release {
				object,
				by,
				consumed,
			})),
			_ => None,
		})
	}

	/// Returns a stream of notifications about installed predicates
	fn predicates(&self) -> impl Stream<Item = Result<PredicateId, Self::Error>> {
		self.events().filter_map(|event| match event {
			Ok(Event::PredicateInstalled { id }) => Some(Ok(id)),
			_ => None,
		})
	}

	/// Returns a stream of all object events that occured on-chain.
	fn events(&self) -> impl Stream<Item = Result<Event, Self::Error>> + Unpin;
}

pub trait ReadOnlyClient {
	type Error;

	/// Retreives the body of an object and its count by its digest.
	fn object(
		&self,
		digest: &Digest,
	) -> impl Future<Output = Result<Option<ActiveObject>, Self::Error>>;

	/// Retreives an object by its reserved uniqueness.
	fn unique(
		&self,
		digest: &Digest,
	) -> impl Future<Output = Result<Option<Object>, Self::Error>>;

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
		signer: &sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		unlock: Option<Expression>,
	) -> impl Future<Output = Result<Object, Self::Error>>;

	/// Unwraps an object into an asset
	fn unwrap(
		&self,
		keypair: &sr25519::Keypair,
		object: &Digest,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Applies a series of state transitions
	/// Returns hashes of objects (Created, Destroyed)
	fn apply(
		&self,
		keypair: &sr25519::Keypair,
		transitions: Vec<Transition<Compact>>,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Installs a new predicate or a group or predicates.
	/// This function accepts either a WASAM binary or a CAR file.
	fn install(
		&self,
		keypair: &sr25519::Keypair,
		wasm_or_car: Vec<u8>,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Transfer an asset.sssss
	fn asset_transfer(
		&self,
		keypair: &sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		recipient: &AccountId32,
	) -> impl Future<Output = Result<(), Self::Error>>;

	/// Transfer native tokens.
	fn native_transfer(
		&self,
		keypair: &sr25519::Keypair,
		amount: Balance,
		recipient: &AccountId32,
	) -> impl Future<Output = Result<(), Self::Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Subxt error: {0:?}")]
	Subxt(#[from] subxt::Error),

	#[error("Runtime error: {0:?}")]
	Runtime(crate::client::model::Error),

	#[error("Dispatch error: {0:?}")]
	DispatchError(DispatchError),
}

impl From<DispatchError> for Error {
	fn from(e: DispatchError) -> Self {
		match e {
			DispatchError::Module(m) => {
				let encoded = m.encode();
				let Ok(decoded) = scale::Decode::decode(&mut &encoded[..]) else {
					return Self::DispatchError(DispatchError::Module(m));
				};
				Self::Runtime(decoded)
			}
			other => Self::DispatchError(other),
		}
	}
}

enum At {
	Latest,
	Block(<SubstrateConfig as Config>::Hash),
}

pub struct Client {
	client: Arc<OnlineClient<SubstrateConfig>>,
	at: At,
}

impl Client {
	/// Construct a new [`OnlineClient`] using default settings which
	/// point to a locally running node on `ws://127.0.0.1:9944`.
	pub async fn new() -> Result<Self, <Self as ReadOnlyClient>::Error> {
		let url = "ws://127.0.0.1:9944";
		Ok(Self {
			client: Arc::new(OnlineClient::from_url(url).await?),
			at: At::Latest,
		})
	}

	/// Construct a new [`OnlineClient`], providing a URL to connect to.
	pub async fn from_url(
		url: impl AsRef<str>,
	) -> Result<Self, <Self as ReadOnlyClient>::Error> {
		Ok(Self {
			client: Arc::new(OnlineClient::from_url(url).await?),
			at: At::Latest,
		})
	}

	/// Construct a new [`OnlineClient`], providing a URL to connect to.
	///
	/// Allows insecure URLs without SSL encryption, e.g. (http:// and ws:// URLs).
	pub async fn from_insecure_url(
		url: impl AsRef<str>,
	) -> Result<Self, <Self as ReadOnlyClient>::Error> {
		Ok(Self {
			client: Arc::new(OnlineClient::from_insecure_url(url).await?),
			at: At::Latest,
		})
	}

	/// Get a version of the client that retreives data at a given block hash.
	pub fn at(
		&self,
		block_hash: impl Into<<SubstrateConfig as Config>::Hash>,
	) -> Self {
		Self {
			client: self.client.clone(),
			at: At::Block(block_hash.into()),
		}
	}

	/// Access storage entries API.
	pub async fn storage(
		&self,
	) -> Result<
		Storage<SubstrateConfig, OnlineClient<SubstrateConfig>>,
		subxt::Error,
	> {
		match self.at {
			At::Latest => self.client.storage().at_latest().await,
			At::Block(hash) => Ok(self.client.storage().at(hash)),
		}
	}

	/// Access the transaction API.
	pub fn tx(&self) -> TxClient<SubstrateConfig, OnlineClient<SubstrateConfig>> {
		self.client.tx()
	}
}

impl AsRef<OnlineClient<SubstrateConfig>> for Client {
	fn as_ref(&self) -> &OnlineClient<SubstrateConfig> {
		&self.client
	}
}
