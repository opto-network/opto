#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod codecs;
pub mod digest;
pub mod env;
pub mod eval;
pub mod expression;
pub mod object;
pub mod predicate;
pub mod query;
pub mod repr;
pub mod transition;

#[cfg(feature = "serde")]
mod serde;

pub use {
	digest::{Digest, Hashable},
	env::Environment,
	eval::{Context, Error as EvalError, Location, Role},
	expression::{Expression, Op},
	object::Object,
	predicate::{Predicate, PredicateId},
	query::ObjectPattern,
	repr::{Compact, Expanded},
	scale as codec,
	transition::Transition,
};

#[cfg(any(test, feature = "test"))]
pub mod test;
