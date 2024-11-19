#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, format};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use {
	super::{Config, GenesisConfig},
	crate::{pallet_objects::Pallet, RuntimeOrigin},
};

pub fn build<T: Config<I>, I>(config: &GenesisConfig<T, I>) {
	todo!();
}
