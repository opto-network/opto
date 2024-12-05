#[cfg(not(feature = "std"))]
use alloc::vec;

use {
	super::*,
	frame::{prelude::*, traits::StaticLookup},
	frame_system::RawOrigin,
};

pub fn unwrap<T: Config<I> + pallet_assets::Config<I>, I: 'static>(
	origin: OriginFor<T>,
	digest: Digest,
) -> DispatchResult {
	let beneficiary = ensure_signed(origin)?;

	// get & remove from state
	let mut object = consume_input::<T, I>(digest)?;

	Pallet::<T, I>::deposit_event(Event::StateTransitioned {
		transition: Transition {
			inputs: vec![digest],
			ephemerals: vec![],
			outputs: vec![],
		},
	});

	// strip optional nonce policy
	object
		.policies
		.retain(|p| p.id != T::NoncePolicyPredicate::get());

	ensure!(
		object.policies.len() == 1
			&& object.policies[0].id == T::CoinPolicyPredicate::get(),
		Error::<T, I>::InvalidAssetObject,
	);

	// check if the unlock predicate is a singular predicate unlockable by
	// the signature of the beneficiary
	ensure!(
		object.unlock.as_ops().len() == 1
			&& object.unlock.as_ops()[0]
				== Op::Predicate(AtRest {
					id: T::DefaultSignatureVerifyPredicate::get(),
					params: beneficiary.encode(),
				}),
		Error::<T, I>::InvalidUnlockForUnwrap,
	);

	let asset_id = <T as pallet_assets::Config<I>>::AssetId::decode(
		&mut object.policies[0].params.as_slice(),
	)
	.map_err(|_| Error::<T, I>::InvalidAssetObject)?;

	let amount = T::Balance::decode(&mut object.data.as_slice())
		.map_err(|_| Error::<T, I>::InvalidAssetObject)?;

	// transfer the asset to the beneficiary from the vault
	pallet_assets::Pallet::<T, I>::transfer(
		RawOrigin::Signed(T::VaultAccount::get()).into(),
		asset_id.into(),
		T::Lookup::unlookup(beneficiary),
		amount,
	)?;

	Ok(())
}
