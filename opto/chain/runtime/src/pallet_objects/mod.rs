pub use {
	model::{ActiveObject, Hold},
	pallet::*,
};
use {
	opto_core::*,
	scale::{Decode, Encode},
	sp_runtime::Vec,
};

pub mod config;
mod dispatch;
mod model;
mod vm;

#[cfg(test)]
mod tests;

#[frame::pallet]
pub mod pallet {
	use {
		super::*,
		crate::interface::Balance,
		config::WeightInfo,
		core::marker::PhantomData,
		frame::{
			prelude::{
				frame_system,
				BuildGenesisConfig,
				DispatchResult,
				OptionQuery,
				OriginFor,
				ValueQuery,
				*,
			},
			traits::ReservableCurrency,
		},
		repr::Compact,
		sp_runtime::AccountId32,
	};

	#[cfg(not(feature = "std"))]
	extern crate alloc;

	#[cfg(not(feature = "std"))]
	use alloc::{vec, vec::Vec};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		/// The contents of the stdpred wasm CAR file.
		///
		/// By default this file is generated when compiling opto-stdpred in
		/// release mode in the target directory of the crate.
		///
		/// It contains all the standard predicates that are used by the runtime
		/// in wasm format.
		pub stdpred: Vec<u8>,

		/// The initial objects that are created in the genesis block.
		pub objects: Vec<Object>,

		/// The initial VRF value of the genesis block.
		pub vrf_seed: Option<Digest>,

		#[serde(skip)]
		pub phantom: PhantomData<(T, I)>,
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {
			config::genesis::build::<T, I>(self);
		}
	}

	#[pallet::config(with_default)]
	pub trait Config<I: 'static = ()>:
		frame_system::Config<AccountId = AccountId32>
		+ pallet_assets::Config<I>
		+ pallet_timestamp::Config
	{
		#[pallet::no_default_bounds]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		/// The maximum number of bytes that predicate code can have.
		type MaximumPredicateSize: Get<u32>;

		/// The maximum number of bytes that a CAR archive can have.
		type MaximumArchiveSize: Get<u32>;

		/// The minimum amount of balance that is required to reserve an object.
		/// Objecs may have higher reservation requirements, but not lower than
		/// this.
		type MinimumReservationDeposit: Get<Balance>;

		/// The minimum time that an object can be reserved for. By default it is
		/// configured to be 2 blocks. Make sure that this value is greater than one
		/// block time because otherwise the reservation will expire before it is
		/// even created and would be useless in secs.
		type MinimumReservationDuration: Get<u64>;

		/// The maximum predicate ID that is reserved for system predicates.
		/// Any predicates installed with IDs equal or less than this value need
		/// to be installed by the root account. All stdpred predicates are
		/// installed during genesis by the root account.
		type ReservedPredicateIds: Get<PredicateId>;

		/// The maximum number of bytes an object can have in its encoded form.
		type MaximumObjectSize: Get<u32>;

		/// The maximum number of policies that an object can have.
		type MaximumObjectPolicies: Get<u8>;

		/// How many historical VRF values to keep in the chain state.
		type HistoryLength: Get<u32>;

		/// The account that holds assets wrapped into objects
		#[pallet::no_default]
		type SystemVaultAccount: Get<Self::AccountId>;

		#[pallet::no_default]
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The predicate that encodes the policy for expressing fungible tokens.
		/// This is the policy that gets attached to objects that represent wrapped
		/// assets as defined in pallet_assets.
		type CoinPolicyPredicate: Get<PredicateId>;
		type NoncePolicyPredicate: Get<PredicateId>;
		type UniquePolicyPredicate: Get<PredicateId>;
		type ReservePolicyPredicate: Get<PredicateId>;
		type DefaultSignatureVerifyPredicate: Get<PredicateId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// A state transition has occured.
		StateTransitioned { transition: Transition<Compact> },

		/// A new predicate was installed
		PredicateInstalled { id: PredicateId },

		/// An object was reserved by an account.
		ObjectReserved {
			object: Digest,
			by: T::AccountId,
			until: model::Timestamp,
		},

		/// An object's reservation was released either by the account that
		/// reserved it or by the system because the reservation period has
		/// expired.
		ReservationReleased {
			object: Digest,
			by: T::AccountId,
			consumed: bool,
		},
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	/// Stores all active objects that exist in the system.
	#[pallet::storage]
	#[pallet::getter(fn object)]
	pub type Objects<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, Digest, model::ActiveObject, OptionQuery>;

	/// Stores the executable WASM code for all installed predicates.
	#[pallet::storage]
	#[pallet::getter(fn predicate)]
	pub type Predicates<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, PredicateId, Vec<u8>, OptionQuery>;

	/// Stores all unique reservations along with the hash of the object
	/// that is using the unique reservation. See the `UniquePolicyPredicate`
	/// and `unique` predicate in the standard predicate library for more
	/// information.
	#[pallet::storage]
	#[pallet::getter(fn unique)]
	pub type Uniques<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, Digest, Digest, OptionQuery>;

	/// Stores the VRF value for a number of recent blocks. This number is
	/// configurable through the `HistoryLength` configuration parameter.
	#[pallet::storage]
	#[pallet::getter(fn vrf)]
	pub type Vrf<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, u32, Digest, OptionQuery>;

	/// Stores the timestamps of the last `HistoryLength` blocks.
	#[pallet::storage]
	#[pallet::getter(fn timestamp)]
	pub type Timestamp<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, u32, u64, OptionQuery>;

	/// Stores the expiration times of all objects that have
	/// been reserved. The time resolution in the system is
	/// on second, expirations are rounded to the nearest second.
	#[pallet::storage]
	#[pallet::getter(fn reservation_expirations)]
	pub type ReservationExpirations<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, model::Timestamp, Vec<Digest>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// The object is too large.
		/// Must be less than `MaximumObjectSize`.
		///
		/// See Config::MaximumObjectSize.
		ObjectTooLarge,

		/// The object has more policies attached to it than the max allowed.
		///
		/// See Config::MaximumObjectPolicies.
		TooManyPolicies,

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

		/// The CAR archive that is being installed is invalid.
		InvalidPredicateArchive,

		/// The CAR archive that is being installed is too large.
		PredicateArchiveTooLarge,

		/// The predicate Id used is reserved for system predicates.
		InvalidPredicateId,

		/// An attempt to wrap zero amount of an asset.
		ZeroWrapAmount,

		/// The object that is being consumed is not found.
		InputObjectNotFound,

		/// The object that is being consumed is reserved.
		InputObjectReserved,

		/// The object has unlock conditions that cannot be used for object
		/// unwrapping.
		InvalidUnlockForUnwrap,

		/// The object that is being unwrapped is not a valid coin.
		InvalidAssetObject,

		/// One or more of the unlock expressions on input objects is not
		/// satisfied.
		UnsatifiedUnlockExpression,

		/// One or more of the policy predicates attached to objects involved in
		/// the transition are not satisfied.
		UnsatifiedPolicy(u8),

		/// An attept to create an object with `UniquePolicyPredicate` that is
		/// that is already taken by another object with the same unique param.
		UniqueAlreadyExists,

		/// An attempt to reserve an object without a reserve policy attached to
		/// it.
		ReservationNotAllowed,

		/// An attempt to reserve an object that is not allowed to be reserved yet.
		ReservationNotAllowedYet,

		/// An attempt to reserve an object that is not allowed to be reserved
		/// anymore.
		ReservationNotAllowedAnymore,

		/// The reservation policy attached to the object has invalid parameters.
		InvalidReservationParameters,
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
			unlock: Option<Expression<Predicate>>,
		) -> DispatchResult {
			dispatch::wrap::<T, I>(origin, asset_id, amount, unlock)
		}

		#[pallet::call_index(2)]
		pub fn unwrap(origin: OriginFor<T>, object: Digest) -> DispatchResult {
			dispatch::unwrap::<T, I>(origin, object)
		}

		#[pallet::call_index(3)]
		pub fn apply(
			origin: OriginFor<T>,
			transitions: Vec<Transition<Compact>>,
		) -> DispatchResult {
			dispatch::apply::<T, I>(origin, transitions)
		}

		#[pallet::call_index(4)]
		pub fn reserve(origin: OriginFor<T>, object: Digest) -> DispatchResult {
			dispatch::reserve::<T, I>(origin, object)
		}
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		/// For each new block, update the VRF value before running any state
		/// transitions. Todo: implement a better way to update the VRF value.
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			dispatch::vrf_update::<T, I>(n)
		}

		/// For each new block, update the history of blocks timestamps.
		///
		/// For each now block, reclaim expired reservations that were not consumed.
		fn on_finalize(n: BlockNumberFor<T>) {
			dispatch::timestamp_update::<T, I>(n);
			dispatch::reclaim_expired_reservations::<T, I>(n);
		}
	}
}
