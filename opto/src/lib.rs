#![cfg_attr(not(feature = "std"), no_std)]

pub use opto_core::{self as core, *};

#[cfg(feature = "p2p")]
pub mod p2p;
