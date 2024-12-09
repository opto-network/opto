use {
	super::*,
	crate::{
		pallet_objects::{self},
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
			policies: vec![Predicate {
				id: UNIQUE_PREDICATE,
				params: unique_tag.clone().to_vec(),
			}],
			unlock: Predicate {
				id: PREIMAGE_PREDICATE,
				params: Digest::compute(b"preimage1").to_vec(),
			}
			.into(),
			data: b"hello world".to_vec(),
		};

		let digest = object.digest();

		assert!(pallet_objects::Objects::<Runtime>::get(digest.clone()).is_none());

		let transition = Transition {
			inputs: vec![],
			ephemerals: vec![],
			outputs: vec![object.clone()],
		};

		assert_ok!(pallet_objects::Pallet::<Runtime>::apply(origin, vec![
			transition.clone()
		]));

		assert_eq!(
			pallet_objects::Objects::<Runtime>::get(digest.clone()),
			Some(StoredObject {
				instance_count: 1,
				object: object.clone(),
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
			policies: vec![Predicate {
				id: UNIQUE_PREDICATE,
				params: unique_tag.clone().to_vec(),
			}],
			unlock: Predicate {
				id: PREIMAGE_PREDICATE,
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
				id: PREIMAGE_PREDICATE,
				params: Digest::compute(b"preimage1").to_vec(),
			}],
			unlock: Predicate {
				id: CONST_PREDICATE,
				params: [1].to_vec(),
			}
			.into(),
			data: b"preimage1".to_vec(),
		};

		// now consume the unique object
		let transition = Transition {
			inputs: vec![digest.clone()],
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
			policies: vec![Predicate {
				id: UNIQUE_PREDICATE,
				params: unique_tag.clone().to_vec(),
			}],
			unlock: Predicate {
				id: PREIMAGE_PREDICATE,
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
			policies: vec![Predicate {
				id: UNIQUE_PREDICATE,
				params: unique_tag.clone().to_vec(),
			}],
			unlock: Predicate {
				id: PREIMAGE_PREDICATE,
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
