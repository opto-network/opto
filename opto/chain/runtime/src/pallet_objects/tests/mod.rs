use {
	super::*,
	crate::{
		interface::AccountId,
		pallet_objects::{self, *},
		Runtime,
	},
};

mod install;
mod utils;
mod wrap;

#[allow(dead_code)]
pub(crate) const VAULT: AccountId = AccountId::new([0u8; 32]);

#[allow(dead_code)]
pub(crate) const COIN_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::CoinPolicyPredicate::get();

#[allow(dead_code)]
pub(crate) const NONCE_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::NoncePolicyPredicate::get();

#[allow(dead_code)]
pub(crate) const DEFAULT_SIGNATURE_PREDICATE: PredicateId =
	<Runtime as pallet_objects::Config>::DefaultSignatureVerifyPredicate::get();

#[allow(dead_code)]
pub(crate) const PREIMAGE_PREDICATE: PredicateId = PredicateId(201);
