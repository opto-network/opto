use {
	super::{model::Hold, Config},
	crate::pallet_objects::{config::WeightInfo, Vrf},
	core::time::Duration,
	frame::{
		prelude::*,
		traits::{ExistenceRequirement, ReservableCurrency, UnixTime, Zero},
	},
	opto_core::{Digest, Hashable, Object},
	scale::Encode,
	sp_core::U256,
	sp_runtime::SaturatedConversion,
	stdpred::util::reserve::Reservation,
};

/// This is invoked as a hook by the runtime on each block before any extrinsics
/// are processed.
///
/// It will calculate the
pub fn vrf_update<T: Config<I>, I: 'static>(
	block_no: BlockNumberFor<T>,
) -> Weight {
	assert!(!block_no.is_zero(), "Block number must be greater than 0");

	let block_no: U256 = block_no.into();
	let block_no: u32 = block_no.try_into() // fix it in about 800 years.
    .expect("block number too large");

	let vrf_history_len = T::HistoryLength::get();
	let previous_vrf = Vrf::<T, I>::get(block_no - 1) // safe because block_no > 0  && history > 1
		.expect("VRF for previous block must exist");

	let previous_timestamp = super::Timestamp::<T, I>::get(block_no - 1)
		.expect("Timestamp for previous block must exist");

	use opto_core::digest::DigestBuilder;
	let mut hasher = Digest::hasher();
	hasher.update(frame_system::Pallet::<T>::parent_hash());
	hasher.update(previous_vrf);
	hasher.update(previous_timestamp.encode());
	hasher.update(frame_system::Pallet::<T>::block_number().encode());
	let new_vrf: Digest = hasher.finalize().into();

	// persist the VRF of the current block
	Vrf::<T, I>::insert(block_no, new_vrf);

	// prune entries older than the max history length
	if block_no >= vrf_history_len {
		Vrf::<T, I>::remove(block_no - vrf_history_len);
	}
	<<T as super::Config<I>>::WeightInfo as WeightInfo>::vrf_init()
}

pub fn timestamp_update<T: Config<I>, I: 'static>(
	block_no: BlockNumberFor<T>,
) -> Weight {
	assert!(!block_no.is_zero(), "Block number must be greater than 0");

	let block_no: U256 = block_no.into();
	let block_no: u32 = block_no.try_into() // fix it in about 800 years.
    .expect("block number too large");

	// persist the timestamp of the current block
	super::Timestamp::<T, I>::insert(
		block_no,
		pallet_timestamp::Pallet::<T>::now().as_millis() as u64,
	);

	// prune entries older than the max history length
	let history_len = T::HistoryLength::get();
	if block_no >= history_len {
		super::Timestamp::<T, I>::remove(block_no - history_len);
	}

	<<T as super::Config<I>>::WeightInfo as WeightInfo>::timestamp_init()
}

pub fn reclaim_expired_reservations<T: Config<I>, I: 'static>(
	block_no: BlockNumberFor<T>,
) -> Weight {
	assert!(!block_no.is_zero(), "Block number must be greater than 0");

	let block_no: U256 = block_no.into();
	let block_no: u32 = block_no.try_into() // fix it in about 800 years.
    .expect("block number too large");

	if block_no == 1 {
		// no reservations to reclaim on block 1, as they must have been placed
		// during genesis. reclaiming reservations on block 1 is not possible
		// because we would have to iterate from zero to timestamp of block 1,
		// which is not possible. The earliest possible block to reclaim is block 2.
		return <<T as super::Config<I>>::WeightInfo as WeightInfo>::reclaim_expired_reservations();
	}

	let current_timestamp = Duration::from_millis(
		super::Timestamp::<T, I>::get(block_no)
			.expect("Timestamp for current block must exist"),
	)
	.as_secs();

	let previous_block_timestamp = Duration::from_millis(
		super::Timestamp::<T, I>::get(block_no - 1)
			.expect("Timestamp for previous block must exist"),
	)
	.as_secs();

	// we are going to iterate at most the number of seconds between two blocks.
	for timestamp in previous_block_timestamp..current_timestamp {
		let expired_reservations =
			super::ReservationExpirations::<T, I>::take(timestamp);

		for digest in expired_reservations {
			super::Objects::<T, I>::mutate_extant(digest, |object| {
				object.reservations.retain(|hold| {
					if hold.until >= timestamp {
						repatriate_deposit::<T, I>(&object.content, hold);
						false
					} else {
						true
					}
				});
			});
		}
	}

	<<T as super::Config<I>>::WeightInfo as WeightInfo>::reclaim_expired_reservations()
}

fn repatriate_deposit<T: Config<I>, I: 'static>(object: &Object, hold: &Hold) {
	use frame::traits::Currency;
	let policy = Reservation::decode(
		&mut object
			.policies
			.iter()
			.find(|p| p.id == stdpred::ids::RESERVE)
			.expect("reservation would not have been created otherwise")
			.params
			.as_slice(),
	)
	.expect("reservation policy param is always valid by this point");

	let deposit = policy.deposit.saturated_into();

	<T as super::Config<I>>::Currency::unreserve(&hold.by, deposit);

	<T as super::Config<I>>::Currency::transfer(
		&hold.by,
		&T::AccountId::decode(&mut policy.payee.as_slice()).expect("infaliable"),
		deposit,
		ExistenceRequirement::AllowDeath,
	)
	.unwrap();

	// emit an event about a released reservation
	super::Pallet::<T, I>::deposit_event(
		super::Event::<T, I>::ReservationReleased {
			object: object.digest(),
			by: hold.by.clone(),
			consumed: false,
		},
	);
}
