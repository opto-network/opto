#![allow(dead_code)]

use {
	super::*,
	crate::{
		interface::AccountId,
		pallet_objects::{self, tests::NONCE_PREDICATE},
		*,
	},
	frame::{testing_prelude::*, traits::fungible::Mutate},
	opto_core::{AtRest, Hashable, Object, PredicateId, Transition},
	sp_keyring::AccountKeyring,
	sp_runtime::MultiAddress,
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

pub fn run_to_block(n: u32) {
	while System::block_number() < n {
		if System::block_number() > 0 {
			pallet_objects::Pallet::<Runtime>::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}

		System::reset_events();
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		pallet_objects::Pallet::<Runtime>::on_initialize(System::block_number());
	}
}
