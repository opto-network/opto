#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeSet, format};
#[cfg(feature = "std")]
use std::collections::BTreeSet;

use {
	super::{Config, GenesisConfig},
	crate::pallet_objects::Pallet,
	frame_system::RawOrigin,
	ipld_nostd::CarReader,
};

pub fn build<T: Config<I>, I: 'static>(config: &GenesisConfig<T, I>) {
	if !config.stdpred.is_empty() {
		install_stdpred::<T, I>(&config.stdpred);
	}
}

fn install_stdpred<T: Config<I>, I: 'static>(car: &[u8]) {
	let reader = CarReader::new(core2::io::Cursor::new(car))
		.expect("failed to load stdpred CAR bytes from genesis");
	let roots: BTreeSet<_> = reader.header().roots().iter().cloned().collect();

	for predicate in reader {
		let (cid, wasm) =
			predicate.expect("Invalid predicate in stdpred CAR in genesis");

		if roots.contains(&cid) {
			continue;
		}

		Pallet::<T, I>::install(RawOrigin::Root.into(), wasm)
			.expect("Failed to install genesis predicate from stdpred CAR");
	}
}
