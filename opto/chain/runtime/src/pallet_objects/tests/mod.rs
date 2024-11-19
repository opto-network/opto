use {
	super::*,
	crate::{
		interface::AccountId,
		pallet_objects::{self, *},
		Runtime,
		System,
	},
	sp_io::TestExternalities,
	sp_runtime::BuildStorage,
};

mod apply;
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
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn at_genesis() -> TestExternalities {
	TestExternalities::new_empty()
}

fn no_genesis() -> TestExternalities {
	let mut ext = TestExternalities::new_empty();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
