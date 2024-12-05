use {
	super::*,
	crate::{
		pallet_objects::{
			self,
			tests::{
				utils::{
					create_asset,
					fixup_nonces_compact,
					mint_asset,
					mint_native_token,
					sign,
				},
				NONCE_PREDICATE,
				VAULT,
			},
			StoredObject,
		},
		*,
	},
	frame::testing_prelude::*,
	opto_core::{repr::Compact, AtRest, Hashable, Object, Op, Transition},
	sp_core::blake2_64,
	sp_keyring::AccountKeyring,
};

#[test]
fn wrap_move_unwrap() {
	// wrap 1000 tokens by alive into an object
	// apply state transition that moves the object to bob
	// unwrap the object by bob

	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let moved_object = Object {
		policies: vec![
			AtRest {
				id: COIN_PREDICATE,
				params: ASSET_ID.encode(),
			},
			AtRest {
				id: NONCE_PREDICATE,
				params: vec![], // will be fixed up later
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

	after_genesis().execute_with(|| {
		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		mint_native_token(
			&AccountKeyring::Charlie.to_account_id(),
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

		let wrapped_object = Object {
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

		let wrapped_object_digest = wrapped_object.digest();

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

		let object = pallet_objects::Objects::<Runtime>::get(wrapped_object_digest)
			.expect("object not found");

		assert_eq!(object.instance_count, 1);
		assert_eq!(object.object, wrapped_object);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![],
					ephemerals: vec![],
					outputs: vec![wrapped_object.clone()],
				},
			}
			.into(),
		);

		// move object to bob
		let mut transition = Transition::<Compact> {
			inputs: vec![wrapped_object_digest],
			outputs: vec![moved_object.clone()],
			ephemerals: vec![],
		};

		fixup_nonces_compact(&mut transition);
		sign(AccountKeyring::Alice, &mut transition);

		let moved_object = transition.outputs[0].clone();
		let moved_object_digest = moved_object.digest();

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Charlie.to_account_id()),
			vec![transition.clone()]
		));

		System::assert_has_event(
			pallet_objects::Event::<Runtime>::StateTransitioned { transition }.into(),
		);

		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(moved_object_digest),
			Some(StoredObject {
				instance_count: 1,
				object: moved_object.clone(),
			})
		);

		// unwrap object by bob
		assert_ok!(pallet_objects::Pallet::<Runtime>::unwrap(
			RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
			moved_object_digest,
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
			TOTAL_SUPPLY - WRAPPED_AMOUNT
		);

		assert_eq!(
			pallet_assets::Pallet::<Runtime>::balance(
				ASSET_ID,
				AccountKeyring::Bob.to_account_id(),
			),
			WRAPPED_AMOUNT
		);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![moved_object.digest()],
					ephemerals: vec![],
					outputs: vec![],
				},
			}
			.into(),
		);

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
fn move_object_wrong_unlock() {
	// wrap 1000 tokens by alive into an object
	// apply state transition that moves the object to bob
	// unwrap the object by bob

	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let moved_object = Object {
		policies: vec![
			AtRest {
				id: COIN_PREDICATE,
				params: ASSET_ID.encode(),
			},
			AtRest {
				id: NONCE_PREDICATE,
				params: vec![], // will be fixed up later
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

	after_genesis().execute_with(|| {
		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.unwrap();

		mint_native_token(
			&AccountKeyring::Charlie.to_account_id(),
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
					params: nonce.to_vec(),
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

		let wrapped_object_digest = wrapped_object.digest();

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

		let object = pallet_objects::Objects::<Runtime>::get(wrapped_object_digest)
			.expect("object not found");

		assert_eq!(object.instance_count, 1);
		assert_eq!(object.object, wrapped_object);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![],
					ephemerals: vec![],
					outputs: vec![wrapped_object],
				},
			}
			.into(),
		);

		// move object to bob
		let mut transition = Transition::<Compact> {
			inputs: vec![wrapped_object_digest],
			outputs: vec![moved_object.clone()],
			ephemerals: vec![],
		};

		fixup_nonces_compact(&mut transition);
		sign(AccountKeyring::Alice, &mut transition);

		// signature for a wrong public key
		transition.ephemerals[0].policies[0].params.reverse();

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(
				RuntimeOrigin::signed(AccountKeyring::Charlie.to_account_id()),
				vec![transition]
			),
			pallet_objects::Error::<Runtime>::UnsatifiedUnlockExpression
		);
	});
}
