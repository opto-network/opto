#[cfg(not(feature = "std"))]
extern crate alloc;

use {
	super::{Config, GenesisConfig, Timestamp, Vrf},
	crate::pallet_objects::Pallet,
	frame_system::RawOrigin,
	opto_core::Digest,
};

pub fn build<T: Config<I>, I: 'static>(config: &GenesisConfig<T, I>) {
	Timestamp::<T, I>::insert(0, 0);

	Vrf::<T, I>::insert(0, config.vrf_seed.unwrap_or(Digest::from([0u8; 32])));

	if !config.stdpred.is_empty() {
		Pallet::<T, I>::install(RawOrigin::Root.into(), config.stdpred.clone())
			.expect("Failed to install stdpred from genesis");
	}
}
