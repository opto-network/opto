//! This module contains various helpers and QoL extensions for working with
//! transitions. All those operations can be done directly on transitions, but
//! this module provides a more ergonomic way to do so.

use {
	opto_core::{
		repr::{Compact, Expanded, Repr},
		Hashable,
		Transition,
	},
	subxt::{tx::Signer, SubstrateConfig},
};

/// Utilities that apply to all transitions.
pub trait TransitionExt
where
	Self: Sized,
{
}

/// Utilities that apply to compact transitions.
pub trait CompactTransitionExt: TransitionExt
where
	Self: Sized,
{
	/// Adds an ephemeral object to the transition with a signature
	/// of the signer. If such an ephemeral object already exists,
	/// then nothing is done.
	fn sign(&mut self, signer: &impl Signer<SubstrateConfig>);
}

/// Utilities that apply to expanded transitions.
pub trait ExpandedTransitionExt: TransitionExt
where
	Self: Sized,
{
	/// Adds an ephemeral object to the transition with a signature
	/// of the signer. If such an ephemeral object already exists,
	/// then nothing is done.
	fn sign(&mut self, signer: &impl Signer<SubstrateConfig>);

	/// Produces a compact version of a state transition. In the compact version
	/// all input object are just references to hashes of the original objects
	/// without their bodies.
	fn compact(self) -> Transition<Compact>;
}

impl<R: Repr> TransitionExt for Transition<R> {}

impl CompactTransitionExt for Transition<Compact> {
	fn sign(&mut self, signer: &impl Signer<SubstrateConfig>) {
		let digest = self.digest_for_signing();
	}
}

impl ExpandedTransitionExt for Transition<Expanded> {
	fn sign(&mut self, signer: &impl Signer<SubstrateConfig>) {
		let digest = self.digest_for_signing();
	}

	fn compact(self) -> Transition<Compact> {
		Transition {
			inputs: self.inputs.into_iter().map(|x| x.digest()).collect(),
			ephemerals: self.ephemerals,
			outputs: self.outputs,
		}
	}
}
