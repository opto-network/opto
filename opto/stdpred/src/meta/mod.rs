//! Those policies have no effect on evaluating state transitions and serve as
//! metadata for offhain processing. They are used to describe things like the
//! details of a job or the contents of someting represented on-chain.

use {crate::utils::*, opto_core::*, opto_onchain::predicate};

/// A blob stored on IPFS.
#[predicate(id = 500, core_crate = opto_core)]
pub fn ipfs(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let len_range = 16..=75;
	ensure!(is_policy(&ctx));
	ensure!(len_range.contains(&params.len()));
	true
}

/// A gossipub topic on the p2p network.
#[predicate(id = 501, core_crate = opto_core)]
pub fn p2ptopic(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let len_range = 32..=32;
	ensure!(is_policy(&ctx));
	ensure!(len_range.contains(&params.len()));
	true
}

/// A multiaddress.
#[predicate(id = 502, core_crate = opto_core)]
pub fn multiaddr(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let len_range = 1..=255;
	ensure!(is_policy(&ctx));
	ensure!(len_range.contains(&params.len()));
	true
}

/// A multiaddress.
#[predicate(id = 503, core_crate = opto_core)]
pub fn memo(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let len_range = 1..=255;
	ensure!(is_policy(&ctx));
	ensure!(len_range.contains(&params.len()));
	true
}

pub mod ids {
	pub use super::{
		ipfs_id as ipfs,
		memo_id as memo,
		multiaddr_id as multiaddr,
		p2ptopic_id as p2ptopic,
	};
}
