use {
	super::DefaultConfig,
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
	pub const ReservedPredicateIds: PredicateId = PredicateId(100000);
}

#[derive_impl(
	frame_system::config_preludes::TestDefaultConfig,
	no_aggregated_types
)]
impl frame_system::DefaultConfig for TestnetDefaultConfig {}

#[frame_support::register_default_impl(TestnetDefaultConfig)]
impl DefaultConfig for TestnetDefaultConfig {
	type HistoryLength = HistoryLength;
	type MaximumArchiveSize = MaximumArchiveSize;
	type MaximumObjectPolicies = MaximumObjectPolicies;
	type MaximumObjectSize = MaximumObjectSize;
	type MaximumPredicateSize = MaximumPredicateSize;
	type ReservedPredicateIds = ReservedPredicateIds;
	#[inject_runtime_type]
	type RuntimeEvent = ();
	type WeightInfo = super::weights::SubstrateWeightInfo;
}
