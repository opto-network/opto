use {
	super::*,
	crate::{
		pallet_objects::{self},
		*,
	},
	frame::testing_prelude::*,
	opto_core::PredicateId,
	sp_keyring::AccountKeyring,
};

#[test]
fn empty_state_has_no_predicates() {
	empty_genesis().execute_with(|| {
		assert_eq!(pallet_objects::Predicates::<Runtime>::iter().count(), 0);
	});
}

#[test]
fn install_predicate_invalid_bytecode() {
	empty_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let bytecode = vec![0x00, 0x01, 0x02, 0x03];

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::install(origin, bytecode),
			pallet_objects::Error::<Runtime>::InvalidPredicateArchive
		);
	});
}

#[test]
fn cannot_install_in_reserved_range() {
	let bytecode = include_bytes!("./assets/101.wasm").to_vec();
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		assert!(
			pallet_objects::Predicates::<Runtime>::get(PredicateId(101)).is_some()
		);
		assert_noop!(
			pallet_objects::Pallet::<Runtime>::install(
				origin.clone(),
				bytecode.clone()
			),
			Error::<Runtime>::InvalidPredicateId
		);
	});
}

#[test]
fn install_predicate_valid_bytecode() {
	let bytecode = include_bytes!("./assets/200001.wasm").to_vec();
	empty_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());

		assert_eq!(pallet_objects::Predicates::<Runtime>::iter().count(), 0);
		assert_ok!(pallet_objects::Pallet::<Runtime>::install(
			origin,
			bytecode.clone()
		));

		assert_eq!(
			pallet_objects::Predicates::<Runtime>::get(PredicateId(200001)),
			Some(bytecode)
		);

		System::assert_last_event(
			pallet_objects::Event::<Runtime>::PredicateInstalled {
				id: PredicateId(200001),
			}
			.into(),
		);
	});
}

#[test]
fn cannot_override_installed_predicate() {
	let bytecode = include_bytes!("./assets/200001.wasm").to_vec();
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());

		if Predicates::<Runtime>::get(PredicateId(200001)).is_none() {
			assert_ok!(pallet_objects::Pallet::<Runtime>::install(
				origin.clone(),
				bytecode.clone()
			));

			System::assert_last_event(
				pallet_objects::Event::<Runtime>::PredicateInstalled {
					id: PredicateId(200001),
				}
				.into(),
			)
		}

		assert!(
			pallet_objects::Predicates::<Runtime>::get(PredicateId(200001)).is_some()
		);
		assert_noop!(
			pallet_objects::Pallet::<Runtime>::install(
				origin.clone(),
				bytecode.clone()
			),
			Error::<Runtime>::PredicateAlreadyExists
		);
	});
}
