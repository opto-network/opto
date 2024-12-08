use {
	super::Config,
	crate::pallet_objects::{config::WeightInfo, Event, Pallet, Vrf},
	frame::{
		prelude::*,
		traits::{UnixTime, Zero},
	},
	opto_core::Digest,
	scale::Encode,
	sp_core::U256,
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

	use opto_core::digest::DigestBuilder;
	let mut hasher = Digest::hasher();
	hasher.update(frame_system::Pallet::<T>::parent_hash());
	hasher.update(pallet_timestamp::Pallet::<T>::get().encode());
	hasher.update(frame_system::Pallet::<T>::block_number().encode());
	hasher.update(previous_vrf);
	let new_vrf: Digest = hasher.finalize().into();

	// persist the VRF of the current block
	Vrf::<T, I>::insert(block_no, new_vrf);

	// prune entries older than the max history length
	if block_no >= vrf_history_len {
		Vrf::<T, I>::remove(block_no - vrf_history_len);
	}

	// emit an event at the beginning of the block
	Pallet::<T, I>::deposit_event(Event::VrfUpdated { vrf: new_vrf });
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
