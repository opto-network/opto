use {
	crate::{
		interface::{self, AccountId},
		pallet_objects,
		Balances,
		Block,
		PalletInfo,
		Runtime,
		RuntimeCall,
		RuntimeEvent,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeOrigin,
		RuntimeTask,
		System,
		VERSION,
	},
	frame::{self, prelude::*, runtime::prelude::*},
	sp_runtime::AccountId32,
};

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
}

/// Implements the types required for the system pallet.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	// Use the account data from the balances pallet
	type AccountData =
		pallet_balances::AccountData<<Runtime as pallet_balances::Config>::Balance>;
	type AccountId = AccountId32;
	type Block = Block;
	type Version = Version;
}

// Implements the types required for the balances pallet.
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
	type AccountStore = System;
}

// Implements the types required for the sudo pallet.
#[derive_impl(pallet_sudo::config_preludes::TestDefaultConfig)]
impl pallet_sudo::Config for Runtime {}

// Implements the types required for the sudo pallet.
#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Runtime {}

// Implements the types required for the transaction payment pallet.
#[derive_impl(pallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pallet_transaction_payment::Config for Runtime {
	// Setting fee as fixed for any length of the call data for demo purposes
	type LengthToFee = FixedFee<1, <Self as pallet_balances::Config>::Balance>;
	type OnChargeTransaction =
		pallet_transaction_payment::FungibleAdapter<Balances, ()>;
	// Setting fee as independent of the weight of the extrinsic for demo purposes
	type WeightToFee = NoFee<<Self as pallet_balances::Config>::Balance>;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Runtime {
	type CreateOrigin = EnsureSigned<interface::AccountId>;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<interface::AccountId>;
	type Freezer = ();
	type WeightInfo = ();
}

parameter_types! {
	// use an account that has no private key. This is the account that will
	// own assets that are wrapped into objects.
	pub const SystemVaultAccount: AccountId = AccountId::new([0u8; 32]);
}

#[derive_impl(pallet_objects::config::TestnetDefaultConfig)]
impl pallet_objects::Config for Runtime {
	type Currency = Balances;
	/// The account that will own assets that are wrapped into objects.
	type SystemVaultAccount = SystemVaultAccount;
}
