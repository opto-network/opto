use {
	super::*,
	crate::{
		pallet_objects::{self, model::ActiveObject},
		*,
	},
	frame::testing_prelude::*,
	sp_keyring::AccountKeyring,
};

#[test]
fn can_create_unique_object() {
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let unique_tag = Digest::compute(b"unqiue1");

		let object = Object {
			policies: vec![
				stdpred::ids::UNIQUE.params(unique_tag),
				stdpred::ids::MEMO.params(b"hello world"),
			],
			unlock: stdpred::ids::BLAKE2B_256
				.params(Digest::compute(b"preimage1"))
				.into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		assert!(pallet_objects::Objects::<Runtime>::get(digest).is_none());

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(origin, vec![
			transition.clone()
		]));

		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: object.clone(),
			})
		);

		System::assert_has_event(
			pallet_objects::Event::<Runtime>::StateTransitioned { transition }.into(),
		);

		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_some());
	});
}

#[test]
fn can_delete_unique_object() {
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let unique_tag = Digest::compute(b"unqiue1");

		let object = Object {
			policies: vec![
				stdpred::ids::MEMO.params(b"hello world"),
				stdpred::ids::UNIQUE.params(unique_tag),
			],
			unlock: Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage1").to_vec(),
			}
			.into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		// craete unique object
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that uniqueness is reserved
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_some());

		let preimage_unlock_obj = Object {
			policies: vec![Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage1").to_vec(),
			}],
			unlock: Predicate {
				id: stdpred::ids::CONSTANT,
				params: [1].to_vec(),
			}
			.into(),
			data: b"preimage1".to_vec(),
		};

		// now consume the unique object
		let transition = Transition {
			inputs: vec![digest],
			ephemerals: vec![preimage_unlock_obj],
			outputs: vec![],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(origin, vec![
			transition.clone()
		]));

		// ensure that uniqueness is removed
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_none());
	});
}

#[test]
fn cant_create_duplicate_unique() {
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let unique_tag = Digest::compute(b"unqiue1");

		let object = Object {
			policies: vec![
				stdpred::ids::MEMO.params(b"hello world"),
				stdpred::ids::UNIQUE.params(unique_tag),
			],
			unlock: Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage1").to_vec(),
			}
			.into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		// craete unique object
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that uniqueness is reserved
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_some());

		// try to create another object with the same uniqueness tag
		let object2 = Object {
			policies: vec![
				stdpred::ids::MEMO.params(b"hello world2"),
				stdpred::ids::UNIQUE.params(unique_tag),
			],
			unlock: Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage2").to_vec(),
			}
			.into(),
			data: b"hello world2".to_vec(),
		};

		// uniqueness should be upheld even if the object is different
		assert_ne!(object2.digest(), digest);

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object2.clone()],
		};

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(
				origin,
				vec![transition.clone()]
			),
			pallet_objects::Error::<Runtime>::UniqueAlreadyExists
		);
	});
}

#[test]
fn create_delete_create_unique() {
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());
		let unique_tag = Digest::compute(b"unqiue2");

		let object = Object {
			policies: vec![
				stdpred::ids::MEMO.params(b"hello world"),
				stdpred::ids::UNIQUE.params(unique_tag),
			],
			unlock: Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage1").to_vec(),
			}
			.into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		let transition_original = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		// craete unique object
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition_original.clone()]
		));

		// ensure that uniqueness is reserved
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_some());

		// ensure that cannot create duplicate unique

		let object2 = Object {
			policies: vec![
				stdpred::ids::MEMO.params(b"hello world2"),
				stdpred::ids::UNIQUE.params(unique_tag),
			],
			unlock: Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage2").to_vec(),
			}
			.into(),
			data: b"hello world2".to_vec(),
		};

		// uniqueness should be upheld even if the object is different
		assert_ne!(object2.digest(), digest);

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object2.clone()],
		};

		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(origin.clone(), vec![
				transition.clone()
			]),
			pallet_objects::Error::<Runtime>::UniqueAlreadyExists
		);

		// now consume the unique object
		let preimage_unlock_obj = Object {
			policies: vec![Predicate {
				id: stdpred::ids::BLAKE2B_256,
				params: Digest::compute(b"preimage1").to_vec(),
			}],
			unlock: Predicate {
				id: stdpred::ids::CONSTANT,
				params: [1].to_vec(),
			}
			.into(),
			data: b"preimage1".to_vec(),
		};

		let transition = Transition {
			inputs: vec![digest],
			ephemerals: vec![preimage_unlock_obj],
			outputs: vec![],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that uniqueness is removed
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_none());

		// now create the object again
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition_original.clone()]
		));

		// ensure that uniqueness is reserved
		assert!(pallet_objects::Uniques::<Runtime>::get(unique_tag).is_some());
	});
}

#[test]
fn cant_consume_object_twice() {
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());

		let object = Object {
			policies: vec![stdpred::ids::MEMO.params(b"hello world")],
			unlock: stdpred::ids::CONSTANT.params(true).into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		let transition_original = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		// craete unique object
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition_original.clone()]
		));

		// ensure that the object is created
		assert!(pallet_objects::Objects::<Runtime>::get(digest).is_some());

		// consume the object
		let transition = Transition {
			inputs: vec![digest],
			ephemerals: vec![],
			outputs: vec![],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that the object is consumed
		assert!(pallet_objects::Objects::<Runtime>::get(digest).is_none());

		// try to consume the object again
		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(origin.clone(), vec![
				transition.clone()
			]),
			pallet_objects::Error::<Runtime>::InputObjectNotFound
		);
	});
}

#[test]
fn can_produce_twice_and_consume_only_twice() {
	// this test will produce an object twice and consume it twice
	// and then ensure that the object is not available for consumption
	// anymore.
	after_genesis().execute_with(|| {
		let alice = AccountKeyring::Alice;
		let origin = RuntimeOrigin::signed(alice.to_account_id());

		let object = Object {
			policies: vec![stdpred::ids::MEMO.params(b"hello world")],
			unlock: stdpred::ids::CONSTANT.params(true).into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		let transition_original = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		// craete one object
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition_original.clone()]
		));

		// ensure that the object is created
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: object.clone(),
			})
		);

		// produce the object again
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition_original.clone()]
		));

		// ensure that the object instance count is incremented
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(digest),
			Some(ActiveObject {
				instance_count: 2,
				reservations: vec![],
				content: object.clone(),
			})
		);

		// consume the object once
		let transition = Transition {
			inputs: vec![digest],
			ephemerals: vec![],
			outputs: vec![],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that the object is consumed
		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(digest),
			Some(ActiveObject {
				instance_count: 1,
				reservations: vec![],
				content: object.clone(),
			})
		);

		// consume the object again
		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(
			origin.clone(),
			vec![transition.clone()]
		));

		// ensure that the object is consumed
		assert!(pallet_objects::Objects::<Runtime>::get(digest).is_none());

		// try to consume the object again
		assert_noop!(
			pallet_objects::Pallet::<Runtime>::apply(origin.clone(), vec![
				transition.clone()
			]),
			pallet_objects::Error::<Runtime>::InputObjectNotFound
		);
	});
}
