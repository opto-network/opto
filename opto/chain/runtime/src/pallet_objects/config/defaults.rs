use {
	super::DefaultConfig,
	frame::{
		deps::frame_support,
		prelude::{frame_system, *},
		runtime::prelude::*,
	},
};

pub struct TestnetDefaultConfig;

parameter_types! {
	pub const MaximumObjectSize: u32 = 1024 * 512; // 512KB
	pub const MaximumPredicateSize: u32 = 1024 * 256; // 256KB
	pub const HistoryLength: u32 = 16_348; // roughtly 24h of blocks
	pub const MaximumObjectPolicies: u8 = 128;
}

#[derive_impl(
	frame_system::config_preludes::TestDefaultConfig,
	no_aggregated_types
)]
impl frame_system::DefaultConfig for TestnetDefaultConfig {}

#[frame_support::register_default_impl(TestnetDefaultConfig)]
impl DefaultConfig for TestnetDefaultConfig {
	type HistoryLength = HistoryLength;
	type MaximumObjectPolicies = MaximumObjectPolicies;
	type MaximumObjectSize = MaximumObjectSize;
	type MaximumPredicateSize = MaximumPredicateSize;
	#[inject_runtime_type]
	type RuntimeEvent = ();
	type WeightInfo = super::weights::SubstrateWeightInfo;
}
