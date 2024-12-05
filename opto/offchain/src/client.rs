use {
	crate::{
		model::api::{storage, tx},
		runtime_types::opto_chain_runtime::pallet_objects::StoredObject,
		AssetId,
		Balance,
		Event,
		MutatingClient,
		ReadOnlyClient,
		StreamingClient,
	},
	futures::Stream,
	opto_core::{
		repr::Compact,
		Digest,
		Expression,
		Object,
		PredicateId,
		Transition,
	},
	std::{collections::HashSet, sync::Arc},
	subxt::{
		storage::Storage,
		tx::TxClient,
		utils::{AccountId32, MultiAddress},
		Config,
		OnlineClient,
		SubstrateConfig,
	},
	tokio::sync::mpsc::unbounded_channel,
	tokio_stream::wrappers::UnboundedReceiverStream,
};

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

type SystemEvent = crate::runtime_types::frame_system::pallet::Event;

type ObjectsEvent =
	crate::runtime_types::opto_chain_runtime::pallet_objects::pallet::Event;

impl ReadOnlyClient for Client {
	type Error = subxt::Error;

	async fn object(
		&self,
		digest: &Digest,
	) -> Result<Option<(Object, u32)>, Self::Error> {
		let key = storage().objects().objects(digest);
		let Some(StoredObject {
			object,
			instance_count,
		}) = self.storage().await?.fetch(&key).await?
		else {
			return Ok(None);
		};
		Ok(Some((object, instance_count)))
	}

	async fn predicate(
		&self,
		id: PredicateId,
	) -> Result<Option<Vec<u8>>, Self::Error> {
		self
			.storage()
			.await?
			.fetch(&storage().objects().predicates(id))
			.await
	}

	async fn asset_balance(
		&self,
		account: &AccountId32,
		asset: AssetId,
	) -> Result<Balance, Self::Error> {
		let key = storage().assets().account(asset, account);
		let Some(asset_account) = self.storage().await?.fetch(&key).await? else {
			return Ok(Balance::default());
		};

		Ok(asset_account.balance)
	}

	async fn native_balance(
		&self,
		account: &AccountId32,
	) -> Result<Balance, Self::Error> {
		let key = storage().system().account(account);
		let Some(res) = self.storage().await?.fetch(&key).await? else {
			return Ok(Balance::default());
		};

		Ok(res.data.free)
	}
}

impl MutatingClient for Client {
	type Error = subxt::Error;

	async fn wrap(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		unlock: Option<Expression>,
	) -> Result<Object, <Self as MutatingClient>::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().objects().wrap(asset_id, amount, unlock),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::Objects(ObjectsEvent::StateTransitioned { transition }) => {
					let Some(object) = transition.outputs.first() else {
						return Err(subxt::Error::Other(format!(
							"Wrapping asset produced an unexpected state transition: \
							 {transition:?}",
						)));
					};
					return Ok(object.clone());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(subxt::Error::Other(format!(
						"Transaction failed: {:?}",
						dispatch_error
					)));
				}
				_ => continue,
			}
		}

		Err(subxt::Error::Other(
			"Transaction failed without giving a reason".to_string(),
		))
	}

	async fn unwrap(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		object: &opto_core::Digest,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().objects().unwrap(*object),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(subxt::Error::Other(format!(
						"Transaction failed: {:?}",
						dispatch_error
					)));
				}
				_ => continue,
			}
		}

		Err(subxt::Error::Other(
			"Transaction failed without giving a reason".to_string(),
		))
	}

	async fn apply(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		transitions: Vec<Transition<Compact>>,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().objects().apply(transitions.clone()),
				signer,
			)
			.await?;

		let mut transitions: HashSet<_> = transitions.into_iter().collect();
		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::Objects(ObjectsEvent::StateTransitioned { transition }) => {
					if transitions.remove(&transition) {
						continue;
					}
				}
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					break;
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(subxt::Error::Other(format!(
						"Transaction failed: {:?}",
						dispatch_error
					)));
				}
				_ => continue,
			}
		}

		if transitions.is_empty() {
			Ok(())
		} else {
			Err(subxt::Error::Other(format!(
				"Not all transitions succeeded. Failed: {transitions:?}"
			)))
		}
	}

	async fn asset_transfer(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		asset_id: AssetId,
		amount: Balance,
		recipient: &AccountId32,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let to: MultiAddress<_, _> = recipient.clone().into();
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().assets().transfer(asset_id, to, amount),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(subxt::Error::Other(format!(
						"Transaction failed: {:?}",
						dispatch_error
					)));
				}
				_ => continue,
			}
		}

		Err(subxt::Error::Other(
			"Transaction failed without giving a reason".to_string(),
		))
	}

	async fn native_transfer(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		amount: Balance,
		recipient: &AccountId32,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let to: MultiAddress<_, _> = recipient.clone().into();
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().balances().transfer_allow_death(to, amount),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(subxt::Error::Other(format!(
						"Transaction failed: {:?}",
						dispatch_error
					)));
				}
				_ => continue,
			}
		}

		Err(subxt::Error::Other(
			"Transaction failed without giving a reason".to_string(),
		))
	}
}

impl StreamingClient for Client {
	type Error = subxt::Error;

	fn transitions(
		&self,
	) -> impl Stream<Item = Result<Transition<Compact>, Self::Error>> {
		let (tx, rx) = unbounded_channel();
		let client = Arc::clone(&self.client);
		tokio::spawn(async move {
			if let Err(e) = recv_loop(client, tx.clone()).await {
				let _ = tx.send(Err(e));
			}
		});

		UnboundedReceiverStream::new(rx)
	}
}

async fn recv_loop<E>(
	client: Arc<OnlineClient<SubstrateConfig>>,
	tx: tokio::sync::mpsc::UnboundedSender<Result<Transition<Compact>, E>>,
) -> Result<(), subxt::Error> {
	let mut subscription = client.blocks().subscribe_finalized().await?;
	while let Some(Ok(block)) = subscription.next().await {
		for event in block.events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::Objects(ObjectsEvent::StateTransitioned { transition }) => {
					let _ = tx.send(Ok(transition));
				}
				_ => continue,
			}
		}
	}
	Ok(())
}
