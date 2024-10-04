//! Opto Standard Predicate Library
//!
//! This library contains the standard predicates that are used in the opto
//! and are defined as part of the Opto standard in the genesis file.
//!
//! The blockchain runtime uses the WASM version of those predicates to evaluate
//! the state transitions. This library provides native implementations of those
//! predicates that can be used in tests and native compute nodes.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
mod utils;

pub mod meta;
pub mod util;

#[derive(Debug)]
pub struct PredicateNotFound;

/// A functor that can be used to instantiate a state transition that uses
/// native implementations of predicates. This is used in tests and native
/// compute nodes. Predicates produced using this factory are more performant,
/// don't require a WASM virtual machine and use less memory. They are however
/// not portable.
pub fn native_impl_factory(
	pred: &opto::repr::AtRest,
) -> Result<opto::PredicateFunctor, PredicateNotFound> {
	Ok(alloc::boxed::Box::new(match pred.id {
		// meta
		meta::ids::ipfs => meta::ipfs,
		meta::ids::p2ptopic => meta::p2ptopic,
		meta::ids::multiaddr => meta::multiaddr,

		// util
		util::ids::constant => util::constant,
		util::ids::nonce => util::nonce,

		_ => return Err(PredicateNotFound),
	}))
}
