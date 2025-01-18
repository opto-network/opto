//! Those policies have no effect on evaluating state transitions and serve as
//! metadata for offhain processing. They are used to describe things like the
//! details of a job or the contents of someting represented on-chain.

use {
	crate::{utils::*, AccountId},
	core::fmt::Debug,
	ipld_nostd::Cid,
	opto_core::*,
	opto_onchain_macros::predicate,
	scale::{Decode, Encode},
};

#[derive(Debug, Encode, Decode, Clone)]
pub struct Meta<T: Debug + Encode + Decode + Clone> {
	pub publisher: AccountId,
	pub payload: T,
}

/// A blob stored on IPFS.
#[predicate(id = 500, core_crate = opto_core)]
pub fn ipfs(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(Meta::<Cid>::decode(&mut &params[..]).is_ok());

	true
}

/// A gossipub topic on the p2p network.
/// user-defined topics are 32 byte hashes.
#[predicate(id = 501, core_crate = opto_core)]
pub fn p2ptopic(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(Meta::<Digest>::decode(&mut &params[..]).is_ok());

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
