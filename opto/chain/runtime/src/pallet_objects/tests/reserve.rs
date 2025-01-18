use {
	super::*,
	crate::{pallet_objects, *},
	core::time::Duration,
	frame::{testing_prelude::*, traits::Time},
	model::{ActiveObject, Hold},
	opto_core::{Hashable, Object},
	sp_keyring::AccountKeyring,
	stdpred::util::reserve::Reservation,
	utils::{
		advance_block_and_exec_fn,
		advance_n_blocks,
		mint_native_token,
		next_block,
	},
};

#[test]
fn can_create_reservation() {
	after_genesis().execute_with(|| {
		assert_ok!(mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			100_000
		));

		let reservable_object = Object {
			policies: vec![
				stdpred::ids::MEMO.params("reservable"),
				stdpred::ids::RESERVE.params(Reservation {
					deposit: 20000,
					payee: AccountKeyring::Charlie.to_account_id().into(),
					duration: Duration::from_secs(60),
					not_after: None,
					not_before: None,
				}),
			],
			unlock: stdpred::ids::CONSTANT.params(1).into(),
			data: b"test-object".to_vec(),
		};

		let digest = reservable_object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![reservable_object.clone()],
		};

		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			vec![transition]
		));

		// ensure that the object was created
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: reservable_object.clone(),
			})
		);

		advance_n_blocks(10);

		// get bob's current native token balance
		let bob_free_balance = pallet_balances::Pallet::<Runtime>::free_balance(
			AccountKeyring::Bob.to_account_id(),
		);
		let bob_reserved_balance =
			pallet_balances::Pallet::<Runtime>::reserved_balance(
				AccountKeyring::Bob.to_account_id(),
			);

		// create a reservation by Bob
		// Bob reserves the object for 60 seconds
		assert_ok!(super::Pallet::<Runtime>::reserve(
			RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
			digest
		));

		// ensure the reservation deposit was fronzen on bob's account
		assert_eq!(
			pallet_balances::Pallet::<Runtime>::free_balance(
				AccountKeyring::Bob.to_account_id(),
			),
			bob_free_balance - 20000
		);

		assert_eq!(
			pallet_balances::Pallet::<Runtime>::reserved_balance(
				AccountKeyring::Bob.to_account_id(),
			),
			bob_reserved_balance + 20000
		);

		let until =
			(Duration::from_millis(pallet_timestamp::Pallet::<Runtime>::now())
				+ Duration::from_secs(60))
			.as_secs();

		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![Hold {
					by: AccountKeyring::Bob.to_account_id(),
					until,
				}],
				content: reservable_object,
			})
		);

		assert_eq!(
			pallet_objects::Pallet::<Runtime>::reservation_expirations(until),
			vec![digest]
		);

		// ensure that reservation event was emitted
		System::assert_has_event(
			Event::<Runtime>::ObjectReserved {
				object: digest,
				by: AccountKeyring::Bob.to_account_id(),
				until,
			}
			.into(),
		);

		// alice can't consume the reserved object
		let transition = Transition {
			inputs: vec![digest],
			ephemerals: vec![],
			outputs: vec![],
		};

		assert_noop!(
			super::Pallet::<Runtime>::apply(
				RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
				vec![transition.clone()]
			),
			Error::<Runtime>::InputObjectReserved
		);

		// bob can consume the reserved object
		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
			vec![transition]
		));

		// ensure the reservation expiration entry is removed
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::reservation_expirations(until),
			vec![]
		);

		// ensure that reservation release event is emitted
		System::assert_has_event(
			Event::<Runtime>::ReservationReleased {
				object: digest,
				by: AccountKeyring::Bob.to_account_id(),
				consumed: true,
			}
			.into(),
		);
	});
}

#[test]
fn cant_reserve_object_without_reserve_policy() {
	after_genesis().execute_with(|| {
		assert_ok!(mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			100_000
		));

		let unreservable_object = Object {
			policies: vec![stdpred::ids::MEMO.params("reservable")],
			unlock: stdpred::ids::CONSTANT.params(1).into(),
			data: b"test-object".to_vec(),
		};

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![unreservable_object.clone()],
		};

		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			vec![transition]
		));

		// bob can't reserve the object
		assert_noop!(
			super::Pallet::<Runtime>::reserve(
				RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
				unreservable_object.digest()
			),
			Error::<Runtime>::ReservationNotAllowed
		);
	});
}

#[test]
fn cant_reserve_object_before_reservation_period() {
	after_genesis().execute_with(|| {
		assert_ok!(mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			100_000
		));

		let now = Duration::from_millis(pallet_timestamp::Pallet::<Runtime>::now());

		let reservable_object = Object {
			policies: vec![
				stdpred::ids::MEMO.params("reservable"),
				stdpred::ids::RESERVE.params(Reservation {
					deposit: 20000,
					payee: AccountKeyring::Charlie.to_account_id().into(),
					duration: Duration::from_secs(60),
					not_after: None,
					not_before: Some(now + Duration::from_secs(60)),
				}),
			],
			unlock: stdpred::ids::CONSTANT.params(1).into(),
			data: b"test-object".to_vec(),
		};

		let digest = reservable_object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![reservable_object.clone()],
		};

		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			vec![transition]
		));

		// ensure that the object was created
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: reservable_object.clone(),
			})
		);

		next_block(); // advance time by 6 seconds

		// still can't create a reservation
		assert_noop!(
			super::Pallet::<Runtime>::reserve(
				RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
				digest
			),
			Error::<Runtime>::ReservationNotAllowedYet
		);

		advance_n_blocks(10); // + 60 seconds

		// now we can create a reservation
		assert_ok!(super::Pallet::<Runtime>::reserve(
			RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
			digest
		));
	});
}

#[test]
fn cant_reserve_object_after_reservation_period() {
	after_genesis().execute_with(|| {
		assert_ok!(mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			100_000
		));

		let now = Duration::from_millis(pallet_timestamp::Pallet::<Runtime>::now());

		let reservable_object = Object {
			policies: vec![
				stdpred::ids::MEMO.params("reservable"),
				stdpred::ids::RESERVE.params(Reservation {
					deposit: 20000,
					payee: AccountKeyring::Charlie.to_account_id().into(),
					duration: Duration::from_secs(60),
					not_after: Some(now + Duration::from_secs(60)),
					not_before: None,
				}),
			],
			unlock: stdpred::ids::CONSTANT.params(1).into(),
			data: b"test-object".to_vec(),
		};

		let digest = reservable_object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![reservable_object.clone()],
		};

		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			vec![transition]
		));

		// ensure that the object was created
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: reservable_object.clone(),
			})
		);

		advance_n_blocks(20); // + 120 seconds

		// can't create a reservation after the reservation period
		assert_noop!(
			super::Pallet::<Runtime>::reserve(
				RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
				digest
			),
			Error::<Runtime>::ReservationNotAllowedAnymore
		);
	});
}

#[test]
fn reservation_expires_when_not_consumed() {
	after_genesis().execute_with(|| {
		assert_ok!(mint_native_token(
			&AccountKeyring::Bob.to_account_id(),
			50_000
		));

		let charlie_balance = pallet_balances::Pallet::<Runtime>::free_balance(
			AccountKeyring::Charlie.to_account_id(),
		);

		let bob_balance = pallet_balances::Pallet::<Runtime>::free_balance(
			AccountKeyring::Bob.to_account_id(),
		);

		let reservable_object = Object {
			policies: vec![
				stdpred::ids::MEMO.params("reservable"),
				stdpred::ids::RESERVE.params(Reservation {
					deposit: 15_000,
					payee: AccountKeyring::Charlie.to_account_id().into(),
					duration: Duration::from_secs(60),
					not_after: None,
					not_before: None,
				}),
			],
			unlock: stdpred::ids::CONSTANT.params(1).into(),
			data: b"test-object".to_vec(),
		};

		let digest = reservable_object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![reservable_object.clone()],
		};

		assert_ok!(super::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(AccountKeyring::Alice.to_account_id()),
			vec![transition]
		));

		next_block();

		// create a reservation by Bob
		// Bob reserves the object for 60 seconds
		assert_ok!(super::Pallet::<Runtime>::reserve(
			RuntimeOrigin::signed(AccountKeyring::Bob.to_account_id()),
			digest
		));

		advance_n_blocks(11); // + 66 seconds

		// finalize the block and run checks before advancing to the next block
		advance_block_and_exec_fn(|| {
			// ensure the reservation expired
			assert_eq!(
				pallet_objects::Pallet::<Runtime>::object(digest),
				Some(ActiveObject {
					instance_count: 1,
					reservations: vec![],
					content: reservable_object.clone(),
				})
			);

			// ensure that the object release event is emitted
			System::assert_has_event(
				Event::<Runtime>::ReservationReleased {
					object: digest,
					by: AccountKeyring::Bob.to_account_id(),
					consumed: false, // still available for consumption
				}
				.into(),
			);

			// ensure the reservation expiration entry is removed
			assert_eq!(
				pallet_objects::Pallet::<Runtime>::reservation_expirations(
					pallet_timestamp::Pallet::<Runtime>::now()
				),
				vec![]
			);

			// ensure that bob lost his deposit
			assert_eq!(
				pallet_balances::Pallet::<Runtime>::free_balance(
					AccountKeyring::Bob.to_account_id(),
				),
				bob_balance - 15_000
			);

			// ensure that bob lost his reserved balance
			assert_eq!(
				pallet_balances::Pallet::<Runtime>::reserved_balance(
					AccountKeyring::Bob.to_account_id(),
				),
				0
			);

			// ensure the deposit is refunded to charlie
			assert_eq!(
				pallet_balances::Pallet::<Runtime>::free_balance(
					AccountKeyring::Charlie.to_account_id(),
				),
				charlie_balance + 15_000
			);
		});
	});
}
