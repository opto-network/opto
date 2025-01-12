use {
	super::*,
	crate::{interface::AccountId, pallet_objects, Runtime},
	sp_io::TestExternalities,
	sp_runtime::BuildStorage,
	utils::run_to_block,
};

mod apply;
mod env;
mod install;
mod reserve;
mod unique;
mod unwrap;
mod utils;
mod wrap;

#[allow(dead_code)]
pub(crate) const VAULT: AccountId = AccountId::new([0u8; 32]);

fn after_genesis() -> TestExternalities {
	let _ = env_logger::try_init();

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();
	pallet_objects::GenesisConfig::<Runtime> {
		// if this file is not present during build and the build fails, ensure
		// that stdpred is built with archive feature first. This will generate the
		// file in the target directory.
		// `cargo build -p opto-stdpred --release --features=archive`
		stdpred: include_bytes!("../../../../../../target/opto-stdpred.car")
			.to_vec(),
		objects: vec![],
		..Default::default()
	}
	.build_storage()
	.expect("Failed to build pallet_objects genesis")
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| run_to_block(1));
	ext
}

fn empty_genesis() -> TestExternalities {
	let _ = env_logger::try_init();

	let mut ext = TestExternalities::new_empty();
	ext.execute_with(|| {
		pallet_objects::Timestamp::<Runtime>::insert(0, 0);
		pallet_objects::Vrf::<Runtime>::insert(0, Digest::from([0u8; 32]));
		run_to_block(1)
	});
	ext
}
