#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "build")]
pub use opto_onchain_builder as builder;
#[cfg(feature = "macros")]
pub use opto_onchain_macros::{predicate, predicates_index};
#[cfg(feature = "stdpred")]
pub use opto_stdpred as stdpred;

pub mod asserts;
pub mod utils;

pub use {asserts::*, opto_core::*};
