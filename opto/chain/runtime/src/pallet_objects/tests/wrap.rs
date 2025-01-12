use {
	super::{utils::create_asset, *},
	crate::{
		pallet_objects::{
			self,
			tests::utils::{mint_asset, mint_native_token},
		},
		*,
	},
	frame::testing_prelude::*,
	opto_core::{Expression, Hashable, Object, Op, Predicate, PredicateId},
	sp_core::blake2_64,
	sp_keyring::AccountKeyring,
};

#[test]
fn empty_state_has_no_objects() {
	empty_genesis().execute_with(|| {
		assert_eq!(pallet_objects::Objects::<Runtime>::iter().count(), 0);
	});
}

#[test]
fn wrap_asset_into_object_default_unlock() {
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

	let expected_object = Object {
		policies: vec![
			Predicate {
				id: stdpred::ids::COIN,
				params: ASSET_ID.encode(),
			},
			Predicate {
				id: stdpred::ids::NONCE,
				params: nonce.to_vec(),
			},
		],
		unlock: vec![Op::Predicate(Predicate {
			id: stdpred::ids::SR25519,
			params: AccountKeyring::Alice.to_account_id().encode(),
		})]
		.try_into()
		.expect("default unlock expression is invalid"),
		data: WRAPPED_AMOUNT.encode(),
	};

	let expected_object_digest = expected_object.digest();

	after_genesis().execute_with(|| {
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(expected_object_digest),
			None
		);

		// This is for paying the fee for wrapping the asset
		let native_balance = mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.expect("minting native token failed");
		assert_eq!(native_balance, NATIVE_TOKEN_BALANCE);

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.expect("asset creation failed");
		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			AccountKeyring::Alice.to_account_id(),
			TOTAL_SUPPLY,
		)
		.expect("asset minting failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let initial_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		let total_supply = pallet_assets::Pallet::<Runtime>::total_supply(1);

		assert_eq!(initial_balance, TOTAL_SUPPLY);
		assert_eq!(total_supply, TOTAL_SUPPLY);

		pallet_objects::Pallet::<Runtime>::wrap(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			ASSET_ID,
			WRAPPED_AMOUNT,
			None,
		)
		.expect("asset wrap failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, WRAPPED_AMOUNT);

		let remaining_balance = pallet_assets::Pallet::<Runtime>::balance(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
		);
		assert_eq!(remaining_balance, TOTAL_SUPPLY - WRAPPED_AMOUNT);

		let object =
			pallet_objects::Objects::<Runtime>::get(expected_object_digest)
				.expect("object not found");

		assert_eq!(object.instance_count, 1);
		assert_eq!(object.content, expected_object);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![],
					ephemerals: vec![],
					outputs: vec![expected_object],
				},
			}
			.into(),
		);

		System::assert_has_event(
			pallet_assets::Event::<Runtime>::Transferred {
				asset_id: 1,
				from: AccountKeyring::Alice.to_account_id(),
				to: VAULT,
				amount: WRAPPED_AMOUNT,
			}
			.into(),
		);
	});
}

#[test]
fn wrap_asset_into_object_custom_unlock() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let custom_unlock_expression: Expression<Predicate> = vec![
		Op::Or,
		Op::Predicate(Predicate {
			id: stdpred::ids::BLAKE2B_256,
			params: b"random-preimage".digest().encode(),
		}),
		Op::Predicate(Predicate {
			id: stdpred::ids::SR25519,
			params: AccountKeyring::Alice.to_account_id().encode(),
		}),
	]
	.try_into()
	.expect("unlock expression is invalid");

	let nonce = blake2_64(
		&[
			AccountKeyring::Alice.to_account_id().encode().as_slice(), //
			0u32.encode().as_slice(), // substrate nonce
		]
		.concat(),
	);

	let expected_object = Object {
		policies: vec![
			Predicate {
				id: stdpred::ids::COIN,
				params: ASSET_ID.encode(),
			},
			Predicate {
				id: stdpred::ids::NONCE,
				params: nonce.to_vec(),
			},
		],
		unlock: custom_unlock_expression.clone(),
		data: WRAPPED_AMOUNT.encode(),
	};

	let expected_object_digest = expected_object.digest();

	after_genesis().execute_with(|| {
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(expected_object_digest),
			None
		);

		// This is for paying the fee for wrapping the asset
		let native_balance = mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.expect("minting native token failed");
		assert_eq!(native_balance, NATIVE_TOKEN_BALANCE);

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.expect("asset creation failed");
		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			AccountKeyring::Alice.to_account_id(),
			TOTAL_SUPPLY,
		)
		.expect("asset minting failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let initial_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		let total_supply = pallet_assets::Pallet::<Runtime>::total_supply(1);

		assert_eq!(initial_balance, TOTAL_SUPPLY);
		assert_eq!(total_supply, TOTAL_SUPPLY);

		pallet_objects::Pallet::<Runtime>::wrap(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			ASSET_ID,
			WRAPPED_AMOUNT,
			Some(custom_unlock_expression.clone()),
		)
		.expect("asset wrap failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, WRAPPED_AMOUNT);

		let remaining_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		assert_eq!(remaining_balance, TOTAL_SUPPLY - WRAPPED_AMOUNT);

		let object =
			pallet_objects::Objects::<Runtime>::get(expected_object_digest)
				.expect("object not found");

		assert_eq!(object.instance_count, 1);
		assert_eq!(object.content, expected_object);

		System::assert_has_event(
			pallet_objects::Event::StateTransitioned {
				transition: Transition {
					inputs: vec![],
					ephemerals: vec![],
					outputs: vec![expected_object],
				},
			}
			.into(),
		);

		System::assert_has_event(
			pallet_assets::Event::<Runtime>::Transferred {
				asset_id: 1,
				from: AccountKeyring::Alice.to_account_id(),
				to: VAULT,
				amount: WRAPPED_AMOUNT,
			}
			.into(),
		);
	});
}

#[test]
fn wrap_asset_into_object_custom_unlock_not_installed_predicate() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 100000000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	let custom_unlock_expression: Expression<Predicate> = vec![
		Op::Or,
		Op::Predicate(Predicate {
			id: PredicateId(9909181),
			params: b"random-preimage".digest().encode(),
		}),
		Op::Predicate(Predicate {
			id: stdpred::ids::SR25519,
			params: AccountKeyring::Alice.to_account_id().encode(),
		}),
	]
	.try_into()
	.expect("unlock expression is invalid");

	let nonce = blake2_64(
		&[
			AccountKeyring::Alice.to_account_id().encode().as_slice(), //
			0u32.encode().as_slice(),
		]
		.concat(),
	);

	let expected_object = Object {
		policies: vec![
			Predicate {
				id: stdpred::ids::COIN,
				params: ASSET_ID.encode(),
			},
			Predicate {
				id: stdpred::ids::NONCE,
				params: nonce.to_vec(),
			},
		],
		unlock: custom_unlock_expression.clone(),
		data: WRAPPED_AMOUNT.encode(),
	};

	let expected_object_digest = expected_object.digest();

	after_genesis().execute_with(|| {
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(expected_object_digest),
			None
		);

		// This is for paying the fee for wrapping the asset
		let native_balance = mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.expect("minting native token failed");
		assert_eq!(native_balance, NATIVE_TOKEN_BALANCE);

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.expect("asset creation failed");
		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			AccountKeyring::Alice.to_account_id(),
			TOTAL_SUPPLY,
		)
		.expect("asset minting failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let initial_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		let total_supply = pallet_assets::Pallet::<Runtime>::total_supply(1);

		assert_eq!(initial_balance, TOTAL_SUPPLY);
		assert_eq!(total_supply, TOTAL_SUPPLY);

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::wrap(
				RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
				ASSET_ID,
				WRAPPED_AMOUNT,
				Some(custom_unlock_expression.clone()),
			),
			pallet_objects::Error::<Runtime>::PredicateNotFound
		);

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let remaining_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		assert_eq!(remaining_balance, TOTAL_SUPPLY);
	});
}

#[test]
fn wrap_asset_into_object_default_unlock_insufficient_balance() {
	const ASSET_ID: u32 = 1;
	const TOTAL_SUPPLY: u64 = 1000;
	const WRAPPED_AMOUNT: u64 = 300000;
	const NATIVE_TOKEN_BALANCE: u64 = 1000;

	TestState::new_empty().execute_with(|| {
		// events are not emitted on the genesis block
		// so here we're setting the block number to 1
		System::set_block_number(1);

		// This is for paying the fee for wrapping the asset
		let native_balance = mint_native_token(
			&AccountKeyring::Alice.to_account_id(),
			NATIVE_TOKEN_BALANCE,
		)
		.expect("minting native token failed");
		assert_eq!(native_balance, NATIVE_TOKEN_BALANCE);

		create_asset(ASSET_ID, AccountKeyring::Alice.to_account_id(), true)
			.expect("asset creation failed");
		mint_asset(
			ASSET_ID,
			AccountKeyring::Alice.to_account_id(),
			AccountKeyring::Alice.to_account_id(),
			TOTAL_SUPPLY,
		)
		.expect("asset minting failed");

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let initial_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		let total_supply = pallet_assets::Pallet::<Runtime>::total_supply(1);

		assert_eq!(initial_balance, TOTAL_SUPPLY);
		assert_eq!(total_supply, TOTAL_SUPPLY);

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::wrap(
				RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
				ASSET_ID,
				WRAPPED_AMOUNT,
				None,
			),
			pallet_assets::Error::<Runtime>::BalanceLow
		);

		let vault_balance = pallet_assets::Pallet::<Runtime>::balance(1, VAULT);
		assert_eq!(vault_balance, 0);

		let remaining_balance = pallet_assets::Pallet::<Runtime>::balance(
			1,
			AccountKeyring::Alice.to_account_id(),
		);
		assert_eq!(remaining_balance, TOTAL_SUPPLY);
	});
}
