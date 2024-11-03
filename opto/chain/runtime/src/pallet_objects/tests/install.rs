use {
	super::*,
	crate::{
		pallet_objects::{self, vm},
		*,
	},
	frame::testing_prelude::*,
	opto_core::PredicateId,
	sp_keyring::AccountKeyring,
};

#[test]
fn empty_state_has_no_predicates() {
	TestState::new_empty().execute_with(|| {
		assert_eq!(pallet_objects::Predicates::<Runtime>::iter().count(), 0);
	});
}

#[test]
fn install_predicate_invalid_bytecode() {
	TestState::new_empty().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let bytecode = vec![0x00, 0x01, 0x02, 0x03];

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::install(origin, bytecode),
			pallet_objects::Error::<Runtime>::InvalidPredicateCode(
				vm::Error::InvalidCode
			)
		);
	});
}

#[test]
fn install_predicate_valid_bytecode() {
	let bytecode = include_bytes!("./assets/101.wasm").to_vec();
	TestState::new_empty().execute_with(|| {
		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());

		assert_eq!(pallet_objects::Predicates::<Runtime>::iter().count(), 0);
		assert_ok!(pallet_objects::Pallet::<Runtime>::install(
			origin,
			bytecode.clone()
		));

		assert_eq!(
			pallet_objects::Predicates::<Runtime>::get(PredicateId(101)),
			Some(bytecode)
		);

		System::assert_last_event(
			pallet_objects::Event::<Runtime>::PredicateInstalled {
				id: PredicateId(101),
			}
			.into(),
		);
	});
}
