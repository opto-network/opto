#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod digest;
pub mod eval;
pub mod expression;
pub mod object;
pub mod predicate;
pub mod repr;
pub mod transition;

pub use {
	digest::{Digest, Hashable},
	eval::{Context, InUse, Location, PredicateFunctor, Role},
	expression::Expression,
	object::Object,
	predicate::{Predicate, PredicateId},
	repr::Repr,
	scale as codec,
	transition::Transition,
};

#[cfg(any(test, feature = "test"))]
pub mod test;
