use {
	super::*,
	model::{objects::model::ActiveObject, storage},
	opto_core::{Digest, Object, PredicateId},
	subxt::utils::AccountId32,
};

impl ReadOnlyClient for Client {
	type Error = Error;

	async fn object(
		&self,
		digest: &Digest,
	) -> Result<Option<ActiveObject>, Self::Error> {
		let key = storage().objects().objects(digest);
		let Some(object) = self.storage().await?.fetch(&key).await? else {
			return Ok(None);
		};

		Ok(Some(object))
	}

	async fn unique(
		&self,
		digest: &Digest,
	) -> Result<Option<Object>, Self::Error> {
		let key = storage().objects().uniques(digest);
		let Some(digest) = self.storage().await?.fetch(&key).await? else {
			return Ok(None);
		};

		self
			.object(&digest)
			.await
			.map(|res| res.map(|object| object.content))
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
			.map_err(Error::Subxt)
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
