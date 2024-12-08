#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(all(feature = "std", feature = "onchain-sdk"))]
// compile_error!(
// 	"Cannot enable both `std` and `onchain-sdk` features at the same time."
// );

pub use opto_core::{self as core, *};
#[cfg(feature = "offchain-sdk")]
pub use opto_offchain::*;
#[cfg(feature = "onchain-sdk")]
pub use opto_onchain as onchain;
#[cfg(feature = "build")]
pub use opto_onchain::builder;
#[cfg(feature = "onchain-sdk")]
pub use opto_onchain::*;
#[cfg(feature = "p2p")]
pub use opto_p2p as p2p;
