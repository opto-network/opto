use {
	super::*,
	crate::{
		pallet_objects::{
			self,
			tests::{
				utils::{
					create_asset,
					install_test_predicates,
					mint_asset,
					mint_native_token,
				},
				NONCE_PREDICATE,
				VAULT,
			},
			StoredObject,
		},
		*,
	},
	frame::testing_prelude::*,
	opto_core::{AtRest, Hashable, Object, Op},
	sp_core::blake2_64,
	sp_keyring::AccountKeyring,
};

#[test]
fn unwrap_object_with_nonce() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let nonce = blake2_64(
		&[
			AccountKeyring::Alice.to_account_id().encode().as_slice(), //
			0u32.encode().as_slice(),
		]
		.concat(),
	);

	let wrapped_object = Object {
		policies: vec![
			AtRest {
				id: COIN_PREDICATE,
				params: ASSET_ID.encode(),
			},
			AtRest {
				id: NONCE_PREDICATE,
				params: nonce.encode(),
			},
		],
		data: WRAPPED_AMOUNT.encode(),
		unlock: vec![Op::Predicate(AtRest {
			id: DEFAULT_SIGNATURE_PREDICATE,
			params: AccountKeyring::Bob.to_account_id().encode(),
		})]
		.try_into()
		.unwrap(),
	};

	TestState::new_empty().execute_with(|| {
		install_test_predicates().unwrap();

		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.unwrap();

		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			VAULT,
			TOTAL_SUPPLY,
		)
		.unwrap();

		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		pallet_objects::Objects::<Runtime>::insert(
			wrapped_object.digest(),
			StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			},
		);

		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 1);
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(wrapped_object.digest()),
			Some(StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			})
		);

		// wrap asset into object
		let origin = RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id());
		let result = pallet_objects::Pallet::<Runtime>::unwrap(
			origin,
			wrapped_object.digest(),
		);
		assert_ok!(result);

		// check that the object was consumed
		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 0);

		// check that the asset was transferred to Bob's account in pallet_assets
		let vault_balance =
			pallet_assets::Pallet::<Runtime>::balance(ASSET_ID, VAULT);
		assert_eq!(vault_balance, TOTAL_SUPPLY - WRAPPED_AMOUNT);

		let bob_balance = pallet_assets::Pallet::<Runtime>::balance(
			ASSET_ID,
			AccountKeyring::Bob.to_account_id(),
		);
		assert_eq!(bob_balance, WRAPPED_AMOUNT);

		// check that there was event signalling the consumption of the object
		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![wrapped_object.digest()],
					ephemerals: vec![],
					outputs: vec![],
				},
			}
			.into(),
		);

		// check that there was event signalling the transfer of the asset
		System::assert_has_event(
			pallet_assets::Event::Transferred {
				asset_id: ASSET_ID,
				from: VAULT,
				to: AccountKeyring::Bob.to_account_id(),
				amount: WRAPPED_AMOUNT,
			}
			.into(),
		);
	});
}

#[test]
fn unwrap_object_without_nonce() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let wrapped_object = Object {
		policies: vec![AtRest {
			id: COIN_PREDICATE,
			params: ASSET_ID.encode(),
		}],
		data: WRAPPED_AMOUNT.encode(),
		unlock: vec![Op::Predicate(AtRest {
			id: DEFAULT_SIGNATURE_PREDICATE,
			params: AccountKeyring::Bob.to_account_id().encode(),
		})]
		.try_into()
		.unwrap(),
	};

	TestState::new_empty().execute_with(|| {
		install_test_predicates().unwrap();

		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.unwrap();

		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			VAULT,
			TOTAL_SUPPLY,
		)
		.unwrap();

		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		pallet_objects::Objects::<Runtime>::insert(
			wrapped_object.digest(),
			StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			},
		);

		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 1);
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(wrapped_object.digest()),
			Some(StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			})
		);

		// wrap asset into object
		let origin = RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id());
		let result = pallet_objects::Pallet::<Runtime>::unwrap(
			origin,
			wrapped_object.digest(),
		);
		assert_ok!(result);

		// check that the object was consumed
		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 0);

		// check that the asset was transferred to Bob's account in pallet_assets
		let vault_balance =
			pallet_assets::Pallet::<Runtime>::balance(ASSET_ID, VAULT);
		assert_eq!(vault_balance, TOTAL_SUPPLY - WRAPPED_AMOUNT);

		let bob_balance = pallet_assets::Pallet::<Runtime>::balance(
			ASSET_ID,
			AccountKeyring::Bob.to_account_id(),
		);
		assert_eq!(bob_balance, WRAPPED_AMOUNT);

		// check that there was event signalling the consumption of the object
		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![wrapped_object.digest()],
					ephemerals: vec![],
					outputs: vec![],
				},
			}
			.into(),
		);

		// check that there was event signalling the transfer of the asset
		System::assert_has_event(
			pallet_assets::Event::Transferred {
				asset_id: ASSET_ID,
				from: VAULT,
				to: AccountKeyring::Bob.to_account_id(),
				amount: WRAPPED_AMOUNT,
			}
			.into(),
		);
	});
}

#[test]
fn unwrap_object_invalid_recipient() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let wrapped_object = Object {
		policies: vec![AtRest {
			id: COIN_PREDICATE,
			params: ASSET_ID.encode(),
		}],
		data: WRAPPED_AMOUNT.encode(),
		unlock: vec![Op::Predicate(AtRest {
			id: DEFAULT_SIGNATURE_PREDICATE,
			params: AccountKeyring::Charlie.to_account_id().encode(),
		})]
		.try_into()
		.unwrap(),
	};

	after_genesis().execute_with(|| {
		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.unwrap();

		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			VAULT,
			TOTAL_SUPPLY,
		)
		.unwrap();

		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		pallet_objects::Objects::<Runtime>::insert(
			wrapped_object.digest(),
			StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			},
		);

		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 1);
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(wrapped_object.digest()),
			Some(StoredObject {
				object: wrapped_object.clone(),
				instance_count: 1,
			})
		);

		// this should fail because the unlock predicate is unlockable by charlie
		// not bob
		assert_noop!(
			pallet_objects::Pallet::<Runtime>::unwrap(
				RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
				wrapped_object.digest(),
			),
			pallet_objects::Error::<Runtime>::InvalidUnlockForUnwrap
		);
	});
}

#[test]
fn wrap_and_unwrap() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	after_genesis().execute_with(|| {
		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.unwrap();

		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			AccountKeyring::Alice.to_account_id(),
			TOTAL_SUPPLY,
		)
		.unwrap();

		let object_nonce = blake2_64(
			&[
				AccountKeyring::Alice.to_account_id().encode().as_slice(), //
				System::account_nonce(AccountKeyring::Alice.to_account_id())
					.encode()
					.as_slice(),
			]
			.concat(),
		);

		let expected_object = Object {
			policies: vec![
				AtRest {
					id: COIN_PREDICATE,
					params: ASSET_ID.encode(),
				},
				AtRest {
					id: NONCE_PREDICATE,
					params: object_nonce.to_vec(),
				},
			],
			unlock: vec![Op::Predicate(AtRest {
				id: DEFAULT_SIGNATURE_PREDICATE,
				params: AccountKeyring::Alice.to_account_id().encode(),
			})]
			.try_into()
			.expect("default unlock expression is invalid"),
			data: WRAPPED_AMOUNT.encode(),
		};

		let expected_object_digest = expected_object.digest();

		// wrap asset into object using default unlocks
		pallet_objects::Pallet::<Runtime>::wrap(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			ASSET_ID,
			WRAPPED_AMOUNT,
			None,
		)
		.expect("asset wrap failed");

		let vault_balance =
			pallet_assets::Pallet::<Runtime>::balance(ASSET_ID, VAULT);
		assert_eq!(vault_balance, WRAPPED_AMOUNT);

		assert_eq!(
			pallet_assets::Pallet::<Runtime>::balance(
				ASSET_ID,
				AccountKeyring::Alice.to_account_id(),
			),
			TOTAL_SUPPLY - WRAPPED_AMOUNT
		);

		let object =
			pallet_objects::Objects::<Runtime>::get(expected_object_digest)
				.expect("object not found");

		assert_eq!(object.instance_count, 1);
		assert_eq!(object.object, expected_object);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![],
					ephemerals: vec![],
					outputs: vec![expected_object.clone()],
				},
			}
			.into(),
		);

		assert_ok!(pallet_objects::Pallet::<Runtime>::unwrap(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			expected_object_digest,
		));

		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 0);
		assert_eq!(
			pallet_assets::Pallet::<Runtime>::balance(ASSET_ID, VAULT),
			0
		);

		assert_eq!(
			pallet_assets::Pallet::<Runtime>::balance(
				ASSET_ID,
				AccountKeyring::Alice.to_account_id(),
			),
			TOTAL_SUPPLY
		);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![expected_object_digest],
					ephemerals: vec![],
					outputs: vec![],
				},
			}
			.into(),
		);

		System::assert_has_event(
			pallet_assets::Event::<Runtime>::Transferred {
				asset_id: ASSET_ID,
				from: VAULT,
				to: AccountKeyring::Alice.to_account_id(),
				amount: WRAPPED_AMOUNT,
			}
			.into(),
		);
	});
}
