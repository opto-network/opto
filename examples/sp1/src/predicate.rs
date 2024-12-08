#![cfg(feature = "onchain")]

use opto::*;

#[predicate(id = 200001)]
pub fn sp1(
	_: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	_: &[u8],
) -> bool {
	false
}
