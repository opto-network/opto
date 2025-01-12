#[cfg(not(feature = "std"))]
use alloc::vec;

use {
	super::*,
	frame::{
		prelude::frame_system,
		traits::{StaticLookup, Zero},
	},
	sp_core::blake2_64,
};

pub fn wrap<T: Config<I>, I: 'static>(
	origin: OriginFor<T>,
	asset_id: T::AssetId,
	amount: T::Balance,
	unlock: Option<Expression<Predicate>>,
) -> DispatchResult {
	let from = ensure_signed(origin.clone())?;
	ensure!(!amount.is_zero(), Error::<T, I>::ZeroWrapAmount);

	let vault = <T as frame_system::Config>::Lookup::unlookup(
		T::SystemVaultAccount::get(), //
	);

	pallet_assets::Pallet::<T, I>::transfer(
		origin,
		asset_id.clone().into(),
		vault.clone(),
		amount,
	)?;

	if let Some(unlock) = unlock.as_ref() {
		// check that all predicates in the unlock expression are installed
		ensure!(
			unlock.as_ops().iter().all(|op| match op {
				Op::Predicate(Predicate { id, .. }) => {
					Predicates::<T, I>::get(*id).is_some()
				}
				_ => true,
			}),
			Error::<T, I>::PredicateNotFound
		);
	}

	// get current account nonce
	let account_id = from.clone();
	let account_nonce = frame_system::Account::<T>::get(&account_id).nonce;
	let account_id_bytes = account_id.encode();
	let account_nonce_bytes = account_nonce.encode();
	let concatenated = [
		account_id_bytes.as_slice(), //
		account_nonce_bytes.as_slice(),
	]
	.concat();
	let nonce = blake2_64(&concatenated);

	let object = Object {
		policies: vec![
			Predicate {
				id: T::CoinPolicyPredicate::get(),
				params: asset_id.encode(),
			},
			Predicate {
				id: T::NoncePolicyPredicate::get(),
				params: nonce.to_vec(),
			},
		],
		unlock: unlock.unwrap_or_else(|| {
			vec![Op::Predicate(Predicate {
				id: T::DefaultSignatureVerifyPredicate::get(),
				params: account_id_bytes,
			})]
			.try_into()
			.expect("default unlock expression is invalid")
		}),
		data: amount.encode(),
	};

	produce_output::<T, I>(object.clone())?;

	Pallet::<T, I>::deposit_event(Event::StateTransitioned {
		transition: Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object],
		},
	});

	Ok(())
}
