//! Opto Standard Predicate Library
//!
//! This library contains the standard predicates that are used in the opto
//! and are defined as part of the Opto standard in the genesis file.
//!
//! The blockchain runtime uses the WASM version of those predicates to evaluate
//! the state transitions. This library provides native implementations of those
//! predicates that can be used in tests and native compute nodes.

#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

use opto_core::env::Environment;

extern crate alloc;

#[macro_use]
mod utils;

pub mod asset;
pub mod crypto;
pub mod env;
pub mod intent;
pub mod meta;
pub mod util;

#[derive(Debug)]
pub struct PredicateNotFound;

/// A functor that can be used to instantiate a state transition that uses
/// native implementations of predicates. This is used in tests and native
/// compute nodes. Predicates produced using this factory are more performant,
/// don't require a WASM virtual machine and use less memory. They are however
/// not portable.
pub fn native_impl_factory<E: Environment + 'static>(
	pred: &opto_core::predicate::Predicate,
) -> Result<opto_core::eval::PredicateFunctor<E>, PredicateNotFound> {
	Ok(alloc::boxed::Box::new(match pred.id {
		// util
		ids::CONSTANT => util::constant,
		ids::NONCE => util::nonce,
		ids::UNIQUE => util::unique,
		ids::RESERVE => util::reserve,

		// crypto
		ids::SR25519 => crypto::sr25519,
		ids::BLAKE2B_256 => crypto::blake2b_256,

		// intents
		ids::OUTPUT => intent::output,
		ids::EPHEMERAL => intent::ephemeral,
		ids::TRANSITION => intent::transition,

		// env
		ids::BEFORE_TIME => env::before_time,
		ids::BEFORE_BLOCK => env::before_block,
		ids::AFTER_TIME => env::after_time,
		ids::AFTER_BLOCK => env::after_block,

		// meta
		ids::IPFS => meta::ipfs,
		ids::P2PTOPIC => meta::p2ptopic,
		ids::MULTIADDR => meta::multiaddr,
		ids::MEMO => meta::memo,

		// economy
		ids::COIN => asset::coin,

		_ => return Err(PredicateNotFound),
	}))
}

pub mod ids {
	opto_onchain_macros::predicates_index!(core_crate = ::opto_core);
}
