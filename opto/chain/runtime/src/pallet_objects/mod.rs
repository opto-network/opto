pub mod config;
pub use pallet::*;

#[frame::pallet]
pub mod pallet {
	use {
		super::config::*,
		frame::prelude::{frame_system, *},
	};

	#[cfg(not(feature = "std"))]
	extern crate alloc;

	#[cfg(not(feature = "std"))]
	use alloc::{vec, vec::Vec};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::config(with_default)]
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_assets::Config<I>
	{
		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);
}
