#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "build")]
pub mod builder;
pub mod utils;

pub use {macros::predicate, opto_core::*};
