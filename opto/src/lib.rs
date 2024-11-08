#![cfg_attr(not(feature = "std"), no_std)]

pub use opto_core::{self as core, *};
#[cfg(feature = "p2p")]
pub use opto_p2p as p2p;
