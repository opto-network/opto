use {
	super::DefaultConfig,
	frame::{
		deps::frame_support,
		prelude::{frame_system, *},
		runtime::prelude::*,
	},
};

pub struct TestnetDefaultConfig;

#[derive_impl(
	frame_system::config_preludes::TestDefaultConfig,
	no_aggregated_types
)]
impl frame_system::DefaultConfig for TestnetDefaultConfig {}

#[frame_support::register_default_impl(TestnetDefaultConfig)]
impl DefaultConfig for TestnetDefaultConfig {
	#[inject_runtime_type]
	type RuntimeEvent = ();
	type WeightInfo = super::weights::SubstrateWeightInfo;
}
