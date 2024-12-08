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
mod unwrap;
mod utils;
mod wrap;

#[allow(dead_code)]
pub(crate) const VAULT: AccountId = AccountId::new([0u8; 32]);

#[allow(dead_code)]
pub(crate) const COIN_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::CoinPolicyPredicate::get();

#[allow(dead_code)]
pub(crate) const NONCE_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::NoncePolicyPredicate::get();

#[allow(dead_code)]
pub(crate) const AFTER_TIME_PREDICATE: PredicateId = PredicateId(402);

#[allow(dead_code)]
pub(crate) const AFTER_BLOCK_PREDICATE: PredicateId = PredicateId(403);

#[allow(dead_code)]
pub(crate) const DEFAULT_SIGNATURE_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::DefaultSignatureVerifyPredicate::get();

#[allow(dead_code)]
pub(crate) const PREIMAGE_PREDICATE: PredicateId = PredicateId(201);

fn after_genesis() -> TestExternalities {
	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();
	let _ = pallet_objects::GenesisConfig::<Runtime> {
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
	let mut ext = TestExternalities::new_empty();
	ext.execute_with(|| {
		pallet_objects::Timestamp::<Runtime>::insert(0, Duration::ZERO);
		pallet_objects::Vrf::<Runtime>::insert(0, Digest::from([0u8; 32]));
		run_to_block(1)
	});
	ext
}
