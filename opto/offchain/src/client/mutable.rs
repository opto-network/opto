use {
	super::*,
	model::{
		objects::pallet::Event as ObjectsEvent,
		system::pallet::Event as SystemEvent,
		tx,
		Event,
	},
	opto_core::{repr::Compact, Expression, Object, Transition},
	std::collections::HashSet,
	subxt::utils::{AccountId32, MultiAddress},
};

impl MutatingClient for Client {
	type Error = Error;

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
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
				Event::Objects(ObjectsEvent::StateTransitioned { transition }) => {
					let Some(object) = transition.outputs.first() else {
						return Err(
							subxt::Error::Other(format!(
								"Wrapping asset produced an unexpected state transition: \
								 {transition:?}",
							))
							.into(),
						);
					};
					return Ok(object.clone());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		Err(
			subxt::Error::Other(
				"Transaction failed without giving a reason".to_string(),
			)
			.into(),
		)
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
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		Err(
			subxt::Error::Other(
				"Transaction failed without giving a reason".to_string(),
			)
			.into(),
		)
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
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
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
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		if transitions.is_empty() {
			Ok(())
		} else {
			Err(Error::Subxt(subxt::Error::Other(format!(
				"Not all transitions succeeded. Failed: {transitions:?}"
			))))
		}
	}

	async fn install(
		&self,
		signer: &crate::signer::sr25519::Keypair,
		wasm_or_car: Vec<u8>,
	) -> Result<(), Self::Error> {
		let tx = self
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().objects().install(wasm_or_car),
				signer,
			)
			.await?;

		let tx_in_block = tx.wait_for_finalized().await?;
		for event in tx_in_block.fetch_events().await?.iter() {
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		Ok(())
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
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		Err(
			subxt::Error::Other(
				"Transaction failed without giving a reason".to_string(),
			)
			.into(),
		)
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
			match event?
				.as_root_event::<Event>()
				.map_err(|e| Error::Subxt(e.into()))?
			{
				Event::System(SystemEvent::ExtrinsicSuccess { .. }) => {
					return Ok(());
				}
				Event::System(SystemEvent::ExtrinsicFailed {
					dispatch_error, ..
				}) => {
					return Err(dispatch_error.into());
				}
				_ => continue,
			}
		}

		Err(
			subxt::Error::Other(
				"Transaction failed without giving a reason".to_string(),
			)
			.into(),
		)
	}
}
