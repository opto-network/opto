use {
	crate::{
		interface,
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
	opto_core::PredicateId,
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
	pub const VaultAccount: interface::AccountId = interface::AccountId::new([0u8; 32]);

	/// Defined in the standard predicate library.
	pub const CoinPolicyPredicate: PredicateId = PredicateId(1000);

	/// Defined in the standard predicate library.
	pub const NoncePolicyPredicate: PredicateId = PredicateId(101);

	/// Defined in the standard predicate library.
	pub const SignatureVerifyPredicate: PredicateId = PredicateId(201);
}

#[derive_impl(pallet_objects::config::TestnetDefaultConfig)]
impl pallet_objects::Config for Runtime {
	/// The predicate id of the `coin` policy.
	/// This policy is part of the standard predicate library and is
	/// known at genesis time.
	type CoinPolicyPredicate = CoinPolicyPredicate;
	/// The predicate id of the signature verification policy that is used when
	/// wrapping assets into objects and not specifying a custom unlock
	/// expression.
	///
	/// By default it is sr25519 signature verification.
	///
	/// When an object is unwrapped into an asset and no extra ephemeral objects
	/// are provided, the `pallet_objects` module will check if the signer of the
	/// transaction is the same as the public key in the object unlock expression
	/// if the unlock expression is Sr25519(signer).
	type DefaultSignatureVerifyPredicate = SignatureVerifyPredicate;
	type NoncePolicyPredicate = NoncePolicyPredicate;
	type VaultAccount = VaultAccount;
}
