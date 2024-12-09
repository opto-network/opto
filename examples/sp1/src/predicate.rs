#![cfg(feature = "onchain")]

use {
	opto::*,
	scale::{Decode, Encode},
};

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct PublicValues;

impl PublicValues {
	pub fn push_input(&mut self, _input: &[u8]) {}

	pub fn push_placeholder(&mut self, _len: Option<u32>) {}
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ProofChallenge {
	pub app_key: [u8; 32],
	pub public_values: PublicValues,
}

#[predicate(id = 200001)]
pub fn sp1(
	_: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	_: &[u8],
) -> bool {
	false
}
