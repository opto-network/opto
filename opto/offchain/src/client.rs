use {
	crate::{
		model::api::{storage, tx},
		runtime_types::opto_chain_runtime::pallet_objects::StoredObject,
		AssetId,
		Balance,
		Event,
		MutatingClient,
		ReadOnlyClient,
	},
	opto_core::{
		repr::Compact,
		Digest,
		Expression,
		Hashable,
		Object,
		PredicateId,
		Transition,
	},
	std::sync::Arc,
	subxt::{
		storage::Storage,
		tx::{Signer, TxClient},
		utils::{AccountId32, MultiAddress},
		Config,
		OnlineClient,
		SubstrateConfig,
	},
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

type AssetsEvent = crate::runtime_types::pallet_assets::pallet::Event;

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
		signer: &impl Signer<SubstrateConfig>,
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
				Event::Objects(ObjectsEvent::ObjectCreated { object }) => {
					return Ok(object);
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
		signer: &impl Signer<SubstrateConfig>,
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
				Event::Objects(ObjectsEvent::ObjectDestroyed { digest }) => {
					if digest == *object {
						return Ok(());
					}
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
		signer: &impl Signer<SubstrateConfig>,
		transitions: Vec<Transition<Compact>>,
	) -> Result<(Vec<Digest>, Vec<Digest>), <Self as MutatingClient>::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().objects().apply(transitions),
				signer,
			)
			.await?;

		let mut created = vec![];
		let mut destroyed = vec![];

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::Objects(ObjectsEvent::ObjectCreated { object }) => {
					created.push(object.digest());
				}
				Event::Objects(ObjectsEvent::ObjectDestroyed { digest }) => {
					destroyed.push(digest);
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

		Ok((created, destroyed))
	}

	async fn asset_transfer(
		&self,
		signer: &impl Signer<SubstrateConfig>,
		asset_id: AssetId,
		amount: Balance,
		recipient: &AccountId32,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().assets().transfer(
					asset_id,
					MultiAddress::Address32(recipient.0),
					amount,
				),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?.as_root_event::<crate::Event>()? {
				Event::Assets(AssetsEvent::Transferred {
					asset_id: transferred_asset_id,
					amount: transferred_amount,
					from,
					to,
				}) => {
					if transferred_asset_id == asset_id
						&& transferred_amount == amount
						&& from == signer.account_id().into()
						&& to == *recipient
					{
						return Ok(());
					}
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
		signer: &impl Signer<SubstrateConfig>,
		amount: Balance,
		recipient: &AccountId32,
	) -> Result<(), <Self as MutatingClient>::Error> {
		let to = MultiAddress::Address32(recipient.0);
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
