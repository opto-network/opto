use {
	super::DefaultConfig,
	crate::interface::Balance,
	frame::{
		deps::frame_support,
		prelude::{frame_system, *},
		runtime::prelude::*,
	},
	opto_core::PredicateId,
};

pub struct TestnetDefaultConfig;

parameter_types! {
	pub const MaximumObjectSize: u32 = 1024 * 512; // 512KB
	pub const MaximumPredicateSize: u32 = 1024 * 256; // 256KB
	pub const MaximumArchiveSize: u32 = 1024 * 256 * 10; // 2.56 MB
	pub const HistoryLength: u32 = 16_348; // roughtly 24h of blocks
	pub const MaximumObjectPolicies: u8 = 128;
	pub const MinimumReservationDeposit: Balance = 10_000;
	pub const MinimumReservationDuration: u64 = 12_000; // 12 seconds = 2 blocks

	/// Defined in the standard predicate library.
	pub const CoinPolicyPredicate: PredicateId = stdpred::ids::COIN;

	/// Defined in the standard predicate library.
	pub const NoncePolicyPredicate: PredicateId = stdpred::ids::NONCE;

	/// Defined in the standard predicate library.
	pub const UniquePolicyPredicate: PredicateId = stdpred::ids::UNIQUE;

	/// Defined in the standard predicate library.
	pub const ReservePolicyPredicate: PredicateId = stdpred::ids::RESERVE;

	/// Defined in the standard predicate library.
	pub const SignatureVerifyPredicate: PredicateId = stdpred::ids::SR25519;

	/// Predicate ids below this value are reserved for the standard library and
	/// system. User installed predicates need to be > this value.
	pub const ReservedPredicateIds: PredicateId = PredicateId(100000);
}

#[derive_impl(
	frame_system::config_preludes::TestDefaultConfig,
	no_aggregated_types
)]
impl frame_system::DefaultConfig for TestnetDefaultConfig {}

#[frame_support::register_default_impl(TestnetDefaultConfig)]
impl DefaultConfig for TestnetDefaultConfig {
	/// The predicate id of the `coin` policy.
	/// This policy is part of the standard predicate library and is
	/// known at genesis time.
	type CoinPolicyPredicate = CoinPolicyPredicate;
	/// The predicate id of the signature verification policy that is used when
	/// wrapping assets into objects and not specifying a custom unlock
	/// expression.
	///
	/// By default it is sr25519 signature verification.
	///
	/// When an object is unwrapped into an asset and no extra ephemeral objects
	/// are provided, the `pallet_objects` module will check if the signer of the
	/// transaction is the same as the public key in the object unlock expression
	/// if the unlock expression is Sr25519(signer).
	type DefaultSignatureVerifyPredicate = SignatureVerifyPredicate;
	type HistoryLength = HistoryLength;
	type MaximumArchiveSize = MaximumArchiveSize;
	type MaximumObjectPolicies = MaximumObjectPolicies;
	type MaximumObjectSize = MaximumObjectSize;
	type MaximumPredicateSize = MaximumPredicateSize;
	type MinimumReservationDeposit = MinimumReservationDeposit;
	type MinimumReservationDuration = MinimumReservationDuration;
	/// The predicate id of the `nonce` policy.
	type NoncePolicyPredicate = NoncePolicyPredicate;
	/// The predicate id of the `reserve` policy.
	type ReservePolicyPredicate = ReservePolicyPredicate;
	type ReservedPredicateIds = ReservedPredicateIds;
	#[inject_runtime_type]
	type RuntimeEvent = ();
	/// The predicate id of the `unique` policy.
	/// Only one object with the same unique policy can exist in the state.
	type UniquePolicyPredicate = UniquePolicyPredicate;
	type WeightInfo = super::weights::SubstrateWeightInfo;
}
