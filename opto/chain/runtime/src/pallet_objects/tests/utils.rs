#![allow(dead_code)]

use {
	super::*,
	crate::{
		interface::AccountId,
		pallet_objects::{self, tests::NONCE_PREDICATE},
		*,
	},
	frame::{
		testing_prelude::*,
		traits::{fungible::Mutate, fungibles::Inspect},
	},
	interface::{AssetId, Balance},
	opto_core::{AtRest, Hashable, Object, PredicateId, Transition},
	repr::{Compact, Expanded},
	sp_core::blake2_64,
	sp_keyring::AccountKeyring,
	sp_runtime::MultiAddress,
	std::vec,
};

pub fn mint_native_token(
	to: &AccountId,
	amount: u64,
) -> Result<u64, DispatchError> {
	pallet_balances::Pallet::<Runtime>::mint_into(to, amount)
}

pub fn create_asset(
	id: u32,
	owner: AccountId,
	sufficient: bool,
) -> DispatchResult {
	match sufficient {
		true => pallet_assets::Pallet::<Runtime>::force_create(
			RuntimeOrigin::root(),
			id,
			MultiAddress::Id(owner),
			true,
			1,
		),
		false => pallet_assets::Pallet::<Runtime>::create(
			RuntimeOrigin::signed(owner.clone()),
			id,
			MultiAddress::Id(owner),
			1,
		),
	}
}

pub fn mint_asset(
	id: u32,
	minter: AccountId,
	beneficiary: AccountId,
	amount: u64,
) -> DispatchResult {
	pallet_assets::Pallet::<Runtime>::mint(
		RuntimeOrigin::signed(minter),
		id,
		MultiAddress::Id(beneficiary),
		amount,
	)
}

pub fn install_test_predicates() -> DispatchResult {
	let alice = AccountKeyring::Alice;
	let alice = RuntimeOrigin::signed(alice.to_account_id());

	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/100.wasm").to_vec(),
	)?;

	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/101.wasm").to_vec(),
	)?;
	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/200.wasm").to_vec(),
	)?;
	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/201.wasm").to_vec(),
	)?;
	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/202.wasm").to_vec(),
	)?;
	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/300.wasm").to_vec(),
	)?;
	pallet_objects::Pallet::<Runtime>::install(
		alice.clone(),
		include_bytes!("./assets/1000.wasm").to_vec(),
	)?;

	Ok(())
}

/// Creates a coin object with the given amount using default unlock predicates.
pub fn create_asset_object(
	amount: Balance,
	asset_id: AssetId,
	owner: AccountId,
) -> Result<Object, DispatchError> {
	// check if the asset exists
	if !pallet_assets::Pallet::<Runtime>::asset_exists(asset_id) {
		create_asset(asset_id, owner.clone(), true)?;
	}

	if pallet_assets::Pallet::<Runtime>::balance(asset_id, &owner) < amount {
		mint_asset(asset_id, owner.clone(), owner.clone(), amount)?;
	}

	let nonce = blake2_64(
		&[
			owner.encode().as_slice(),
			System::account_nonce(&owner).encode().as_slice(),
		]
		.concat(),
	);

	pallet_objects::Pallet::<Runtime>::wrap(
		RuntimeOrigin::signed(owner.clone()),
		asset_id,
		amount,
		None,
	)?;

	let expected_object = Object {
		policies: vec![
			AtRest {
				id: COIN_PREDICATE,
				params: asset_id.encode(),
			},
			AtRest {
				id: NONCE_PREDICATE,
				params: nonce.to_vec(),
			},
		],
		unlock: AtRest {
			id: DEFAULT_SIGNATURE_PREDICATE,
			params: owner.encode(),
		}
		.into(),
		data: amount.encode(),
	};

	let expected_hash = expected_object.digest();
	let object = pallet_objects::Pallet::<Runtime>::object(expected_hash)
		.ok_or_else(|| DispatchError::from("wrapped asset object not found"))?;

	assert!(object.instance_count > 0);

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

	Ok(expected_object)
}

pub fn next_block() {
	run_to_block(System::block_number() + 1);
}

pub fn run_to_block(n: u32) {
	while System::block_number() < n {
		if System::block_number() > 0 {
			pallet_objects::Pallet::<Runtime>::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
			pallet_timestamp::Pallet::<Runtime>::on_finalize(System::block_number());
			pallet_objects::Pallet::<Runtime>::on_finalize(System::block_number());
		}

		System::reset_events();
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());

		// in tests each block is 6 seconds (6000 milliseconds)
		pallet_timestamp::Pallet::<Runtime>::set_timestamp(
			(System::block_number() * 6_000) as u64,
		);

		pallet_objects::Pallet::<Runtime>::on_initialize(System::block_number());
	}
}

pub fn fixup_nonces_expanded(transition: &mut Transition<Expanded>) {
	use blake2::{digest::consts::U8, Digest};

	type Hasher = blake2::Blake2b<U8>;

	fn hash_concat(elems: &[&[u8]]) -> u64 {
		let mut hasher = Hasher::default();
		for elem in elems {
			hasher.update(elem);
		}
		u64::from_le_bytes(hasher.finalize().into())
	}

	let mut hasher = Hasher::default();
	for input in transition.inputs.iter() {
		hasher.update(input.digest());
	}

	let inputs_hash: [u8; 8] = hasher.finalize().into();
	for (ix, object) in transition.outputs.iter_mut().enumerate() {
		if let Some(nonce_policy) =
			object.policies.iter_mut().find(|p| p.id == NONCE_PREDICATE)
		{
			let nonce =
				hash_concat(&[&inputs_hash, (ix as u64).to_le_bytes().as_slice()]);
			nonce_policy.params = nonce.to_le_bytes().to_vec();
		}
	}
}

pub fn fixup_nonces_compact(transition: &mut Transition<Compact>) {
	use blake2::{digest::consts::U8, Digest};

	type Hasher = blake2::Blake2b<U8>;

	fn hash_concat(elems: &[&[u8]]) -> u64 {
		let mut hasher = Hasher::default();
		for elem in elems {
			hasher.update(elem);
		}
		u64::from_le_bytes(hasher.finalize().into())
	}

	let mut hasher = Hasher::default();
	for input in transition.inputs.iter() {
		hasher.update(input);
	}

	let inputs_hash: [u8; 8] = hasher.finalize().into();
	for (ix, object) in transition.outputs.iter_mut().enumerate() {
		if let Some(nonce_policy) =
			object.policies.iter_mut().find(|p| p.id == NONCE_PREDICATE)
		{
			let nonce =
				hash_concat(&[&inputs_hash, (ix as u64).to_le_bytes().as_slice()]);
			nonce_policy.params = nonce.to_le_bytes().to_vec();
		}
	}
}

pub fn sign(keyring: AccountKeyring, transition: &mut Transition<Compact>) {
	let message = transition.digest_for_signing();
	let signature = keyring.sign(message.as_slice());
	transition.ephemerals.push(Object {
		policies: vec![AtRest {
			id: DEFAULT_SIGNATURE_PREDICATE,
			params: keyring.to_account_id().encode(),
		}],
		unlock: AtRest {
			id: PredicateId(100), // const
			params: vec![1],
		}
		.into(),
		data: signature.to_vec(),
	});
}
