//! This module contains various helpers and QoL extensions for working with
//! transitions. All those operations can be done directly on transitions, but
//! this module provides a more ergonomic way to do so.
//!
//! The helpers in this module do not cover all possible operations on
//! transitions, here we are prioritizing convention and ergonomics over
//! completeness.

use opto_core::{
	predicate,
	repr::{Compact, Expanded, Repr},
	AtRest,
	Hashable,
	Transition,
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
	fn sign(&mut self, signer: &subxt_signer::sr25519::Keypair);
}

/// Utilities that apply to expanded transitions.
trait ExpandedTransitionExt: TransitionExt
where
	Self: Sized,
{
	/// Converts an expanded transition to a compact transition.
	fn compact(self) -> Transition<Compact>;
}

impl<R: Repr> TransitionExt for Transition<R> {}

impl CompactTransitionExt for Transition<Compact> {
	fn sign(&mut self, signer: &subxt_signer::sr25519::Keypair) {
		let predicate_id = opto_stdpred::crypto::ids::sr25519;
		let pubkey = signer.public_key();
		let predicate = predicate::AtRest {
			id: predicate_id,
			params: pubkey.as_ref().to_vec(),
		};

		// first check if we already have a signature for this signer
		if self
			.ephemerals
			.iter()
			.any(|obj| obj.policies.iter().any(|p| p == &predicate))
		{
			return;
		}

		let signature = signer.sign(self.digest_for_signing().as_slice());

		// if not, then add a new ephemeral object with the signature
		let signature_object = crate::Object {
			policies: vec![predicate],
			unlock: AtRest {
				id: opto_stdpred::util::ids::constant,
				params: [1].to_vec(),
			}
			.into(),
			data: signature.as_ref().to_vec(),
		};

		self.ephemerals.push(signature_object);
	}
}

impl ExpandedTransitionExt for Transition<Expanded> {
	fn compact(self) -> Transition<Compact> {
		Transition {
			inputs: self.inputs.into_iter().map(|x| x.digest()).collect(),
			ephemerals: self.ephemerals,
			outputs: self.outputs,
		}
	}
}
