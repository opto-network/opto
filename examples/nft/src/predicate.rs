//! NFT Predicate
//!
//! This example demonstrates how to build a non-fungible token (NFT) predicate
//! and its interactions with the runtime.
//!
//! An NFT needs to preserve the following properties:
//! - uniqueness: Each NFT has a `unique` policy attached to it that guarantees
//!   that there is only one instance of it in the system at any point in time.
//!   The uniqueness value is the hash of the

use {
	opto::*,
	scale::{ConstEncodedLen, Decode, Encode, MaxEncodedLen},
};

/// A serialized version of this struct is the contents of the `nft` policy.
/// The uniqueness of an nft is the hash of this struct.
#[derive(Debug, Encode, Decode, MaxEncodedLen)]
pub struct NftIdentity {
	pub mint: Digest,
	pub tag: Digest,
	pub mutable: bool,
}
impl ConstEncodedLen for NftIdentity {}

#[predicate(id = 200020)]
pub fn nft(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	mut params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(params.len() == NftIdentity::max_encoded_len());

	let identity = NftIdentity::decode(&mut params)
		.expect("params len is equal to this struct's constant encoded len");

	// we require that nfts are unique per mint/tag pair
	ensure!(uniqueness_equals(
		&ctx,
		&Digest::compute_concat(&[
			identity.mint.as_slice(), // identifies the source of this NFT
			identity.tag.as_slice(),  // unique per instance of the NFT for this mint
		])
	));

	if ctx.location == Location::Input {
		// if the NFT is an input, then it should be valid because it must have
		// passed all the checks in the previous state when it was an output.
		return true;
	}

	// if an NFT is an output of a state transition, then it must have been
	// either included as an input or its mint must be included as an
	// input to the state transition.

	// Begin by validating the case when it was included as an input.
	for input in transition.inputs.iter() {
		let Some(instance) = input
			.policies
			.iter()
			.find(|p| p.id == ctx.predicate_id())
			.and_then(|p| NftIdentity::decode(&mut p.params.as_ref()).ok())
			.filter(|id| id.mint == identity.mint && id.tag == identity.tag)
		else {
			// this is some other unrelated NFT on the input.
			continue;
		};

		// getting here `instance` is referring to this NFT in the input state.
		// ensure that all properties are preserved.

		if !instance.mutable {
			// if nft is immutable then its data, mutability or policies
			// must not change during state transition. Only ownership
			// can change as a change in the unlock expression.
			if input.data != ctx.object.data
				|| !equivalent_policies(input, ctx.object)
				|| instance.mutable != identity.mutable
			{
				return false;
			}
		}

		return true; // all good, was on input, all properties preserved
	}

	// getting here means that this NFT was not an input to this state
	// transition. This means that the only valid way for this NFT to be
	// an output is if it was minted in this state transition. This is expressed
	// by having the mint object included as an input in the state transition. If
	// the state transition is able to unlock the mint object, then it means that
	// it has permissions to mint NFTs of its type.

	// find an input object that is the mint of this NFT
	transition.inputs.iter().any(|input| {
		input
			.policies
			.iter()
			.any(|p| p.id == ids::NFT_MINT && p.params == identity.mint.as_slice())
	})
}

/// Policies of this type are allowed to mint NFTs.
/// All NFTs minted using objects with this policy must have this mint
/// as a parameter.
///
/// There cannot be two objects with this policy/param combination in the same
/// time in the system. This is guaranteed by requiring that the uniqueness
/// policy is attached to the mint object.
#[predicate(id = 200021)]
pub fn nft_mint(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	mut params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(params.len() == Digest::max_encoded_len());

	ensure!(uniqueness_equals(
		&ctx,
		&Digest::decode(&mut params)
			.expect("params len is equal to digest's max encoded len")
	));

	true
}

pub mod ids {
	opto::predicates_index!();
}
