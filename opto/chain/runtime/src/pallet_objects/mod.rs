pub use pallet::*;
use {
	frame::derive::TypeInfo,
	opto_core::*,
	scale::{Decode, Encode},
	sp_runtime::Vec,
};

pub mod config;
mod dispatch;
mod vm;

#[cfg(test)]
mod tests;

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo)]
pub struct StoredObject {
	/// The total number of copies of the object that are stored.
	/// Each time an object is consumed, this value is decremented by 1.
	/// When the value reaches 0, the object is removed from the storage.
	pub instance_count: u32,
	pub object: Object<AtRest, Vec<u8>>,
}

#[frame::pallet]
pub mod pallet {
	use {
		super::{config::*, *},
		frame::prelude::{frame_system, *},
	};

	#[cfg(not(feature = "std"))]
	extern crate alloc;

	#[cfg(not(feature = "std"))]
	use alloc::{vec, vec::Vec};

	use frame::prelude::{DispatchResult, OriginFor};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::config(with_default)]
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_assets::Config<I>
	{
		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		/// The maximum number of bytes that predicate code can have.
		type MaximumPredicateSize: Get<u32>;

		/// The maximum number of bytes an object can have in its encoded form.
		type MaximumObjectSize: Get<u32>;

		/// The account that holds assets wrapped into objects
		#[pallet::no_default]
		type VaultAccount: Get<Self::AccountId>;

		/// The predicate that encodes the policy for expressing fungible tokens.
		/// This is the policy that gets attached to objects that represent wrapped
		/// assets as defined in pallet_assets.
		#[pallet::no_default]
		type CoinPolicyPredicate: Get<PredicateId>;

		#[pallet::no_default]
		type NoncePolicyPredicate: Get<PredicateId>;

		#[pallet::no_default]
		type DefaultSignatureVerifyPredicate: Get<PredicateId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		ObjectCreated { object: Object<AtRest, Vec<u8>> },
		ObjectDestroyed { digest: Digest },
		PredicateInstalled { id: PredicateId },
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::storage]
	#[pallet::getter(fn object)]
	pub type Objects<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, Digest, StoredObject, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn predicate)]
	pub type Predicates<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, PredicateId, Vec<u8>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// The object is too large.
		/// Must be less than `MaximumObjectSize`.
		ObjectTooLarge,

		/// Predicate code is too large.
		/// Must be less than `MaximumPredicateSize`.
		PredicateTooLarge,

		/// Predicate not found.
		/// The predicate with the given ID is not installed.
		PredicateNotFound,

		/// Predicate already exists.
		/// The predicate with the given ID is already installed.
		PredicateAlreadyExists,

		/// The predicate that is being installed has invalid
		/// wasm bytecode. It could be missing some exports or
		/// exporting wrong signatures or the WASM code itself is
		/// not a valid WASM code.
		InvalidPredicateCode(vm::Error),

		/// An attempt to wrap zero amount of an asset.
		ZeroWrapAmount,

		/// The object that is being consumed is not found.
		InputObjectNotFound,

		/// The object has unlock conditions that cannot be used for object
		/// unwrapping.
		InvalidUnlockForUnwrap,

		/// The object that is being unwrapped is not a valid coin.
		InvalidAssetObject,
	}

	/// The pallet's dispatchable extrinisicts.
	#[pallet::call(weight(<T as Config<I>>::WeightInfo))]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::call_index(0)]
		pub fn install(origin: OriginFor<T>, bytecode: Vec<u8>) -> DispatchResult {
			dispatch::install::<T, I>(origin, bytecode)
		}

		#[pallet::call_index(1)]
		pub fn wrap(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			amount: T::Balance,
			unlock: Option<Expression<AtRest>>,
		) -> DispatchResult {
			dispatch::wrap::<T, I>(origin, asset_id, amount, unlock)
		}

		#[pallet::call_index(2)]
		pub fn unwrap(origin: OriginFor<T>, object: Digest) -> DispatchResult {
			dispatch::unwrap::<T, I>(origin, object)
		}
	}
}
