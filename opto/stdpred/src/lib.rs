//! Opto Standard Predicate Library
//!
//! This library contains the standard predicates that are used in the opto
//! and are defined as part of the Opto standard in the genesis file.
//!
//! The blockchain runtime uses the WASM version of those predicates to evaluate
//! the state transitions. This library provides native implementations of those
//! predicates that can be used in tests and native compute nodes.

#![cfg_attr(not(test), no_std)]

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
		util::ids::constant => util::constant,
		util::ids::nonce => util::nonce,

		// crypto
		crypto::ids::sr25519 => crypto::sr25519,
		crypto::ids::blake2b_256 => crypto::blake2b_256,

		// intents
		intent::ids::output => intent::output,
		intent::ids::ephemeral => intent::ephemeral,
		intent::ids::input => intent::input,

		// env
		env::ids::before_time => env::before_time,
		env::ids::before_block => env::before_block,
		env::ids::after_time => env::after_time,
		env::ids::after_block => env::after_block,

		// meta
		meta::ids::ipfs => meta::ipfs,
		meta::ids::p2ptopic => meta::p2ptopic,
		meta::ids::multiaddr => meta::multiaddr,
		meta::ids::memo => meta::memo,

		// economy
		asset::ids::coin => asset::coin,

		_ => return Err(PredicateNotFound),
	}))
}
