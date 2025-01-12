use {
	super::{
		after_genesis,
		utils::{create_asset_object, mint_native_token},
	},
	crate::{
		interface::AssetId,
		pallet_objects::{
			self,
			tests::utils::{fixup_nonces_compact, run_to_block, sign},
			Error,
		},
		Runtime,
		RuntimeOrigin,
		System,
		Timestamp,
	},
	core::time::Duration,
	frame::traits::UnixTime,
	frame_support::{assert_noop, assert_ok},
	opto_core::{Hashable, Object, PredicateIdExt, Transition},
	sp_keyring::AccountKeyring,
};

#[test]
fn time_based_unlock() {
	after_genesis().execute_with(|| {
		const ASSET_ID: AssetId = 10;
		// in the test runtime the timestamp is always block_no * 6 seconds
		const UNLOCK_BLOCK: u32 = 16;
		const UNLOCK_TIME: Duration = Duration::from_secs(UNLOCK_BLOCK as u64 * 6);

		let account = AccountKeyring::Alice.to_account_id();

		// enough to pay for transaction fees
		mint_native_token(&account, 1000).unwrap();
		let coin = create_asset_object(5000, ASSET_ID, account.clone()).unwrap();

		// now time-lock this object to timestamp 2001
		let mut transition = Transition {
			inputs: vec![coin.digest()],
			ephemerals: vec![], // signature will be attached later
			outputs: vec![Object {
				unlock: stdpred::ids::AFTER_TIME.params(UNLOCK_TIME).into(),
				..coin
			}],
		};

		fixup_nonces_compact(&mut transition);
		sign(AccountKeyring::Alice, &mut transition);

		let time_locked_object_digest = transition.outputs[0].digest();

		// apply the transition
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(account.clone()),
			vec![transition],
		));

		// check that the time locked object exists in storage
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(time_locked_object_digest)
				.unwrap()
				.instance_count,
			1
		);

		// we are still at block 1, so time is 1000,
		// we should not be able to unlock the object

		let transition = Transition {
			inputs: vec![time_locked_object_digest],
			ephemerals: vec![],
			outputs: vec![],
		};

		assert!(System::block_number() < UNLOCK_BLOCK);
		assert!(Timestamp::now() < UNLOCK_TIME);

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(
				RuntimeOrigin::signed(account.clone()),
				vec![transition.clone()]
			),
			Error::<Runtime>::UnsatifiedUnlockExpression
		);

		run_to_block(UNLOCK_BLOCK + 1);

		assert!(System::block_number() >= UNLOCK_BLOCK);
		assert!(Timestamp::now() >= UNLOCK_TIME);

		// now we are at block 16, so time is 16000,
		// we should be able to unlock the object

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			RuntimeOrigin::signed(account),
			vec![transition]
		));

		// check that the time locked object is removed from storage
		assert_eq!(
			pallet_objects::Pallet::<Runtime>::object(time_locked_object_digest),
			None
		);
	});
}

#[test]
fn block_no_based_unlock() {}
