//! This module contains various helpers and QoL extensions for working with
//! transitions. All those operations can be done directly on transitions, but
//! this module provides a more ergonomic way to do so.
//!
//! The helpers in this module do not cover all possible operations on
//! transitions, here we are prioritizing convention and ergonomics over
//! completeness.

use {
	opto_core::{
		repr::{Compact, Expanded},
		Hashable,
		Object,
		Predicate,
		Transition,
	},
	subxt_signer::sr25519,
};

macro_rules! sign_with_sr25519 {
	($transition:expr, $signer:expr) => {
		let predicate_id = opto_stdpred::ids::SR25519;
		let pubkey = $signer.public_key();
		let predicate = opto_core::Predicate {
			id: predicate_id,
			params: pubkey.as_ref().to_vec(),
		};

		let signature = $signer
			.sign($transition.digest_for_signing().as_slice())
			.as_ref()
			.to_vec();

		// first check if we already have a signature for this signer
		if let Some(signature_object) = $transition
			.ephemerals
			.iter_mut()
			.find(|obj| obj.policies.iter().any(|p| p == &predicate))
		{
			// there's already a signature for this signer attached
			// to the transition. Update it in case the transition changed
			// since the last time it was signed.
			signature_object.data = signature;
		} else {
			// if not, then add a new ephemeral object with the signature
			let signature_object = Object {
				policies: vec![predicate],
				unlock: Predicate {
					id: opto_stdpred::ids::CONSTANT,
					params: [1].to_vec(),
				}
				.into(),
				data: signature,
			};

			$transition.ephemerals.push(signature_object);
		}
	};
}

macro_rules! set_nonces {
	($transition:expr, $digest: expr) => {
		use blake2::{digest::consts::U8, Digest};
		type Hasher = blake2::Blake2b<U8>;
		fn hash_concat(elems: &[&[u8]]) -> u64 {
			let mut hasher = Hasher::default();
			for elem in elems {
				hasher.update(elem);
			}
			u64::from_le_bytes(hasher.finalize().into())
		}

		let digest_fn = $digest;
		let mut hasher = Hasher::default();
		for input in $transition.inputs.iter() {
			hasher.update(digest_fn(input));
		}

		let inputs_hash: [u8; 8] = hasher.finalize().into();
		for (ix, object) in $transition.outputs.iter_mut().enumerate() {
			if let Some(nonce_policy) = object
				.policies
				.iter_mut()
				.find(|p| p.id == opto_stdpred::ids::NONCE)
			{
				let nonce = hash_concat(&[
					&inputs_hash, //
					(ix as u64).to_le_bytes().as_slice(),
					object.unlock.digest().as_slice(),
					&object.data,
				]);
				nonce_policy.params = nonce.to_le_bytes().to_vec();
			}
		}
	};
}

/// Utilities that apply to compact transitions.
pub trait CompactTransitionExt
where
	Self: Sized,
{
	fn sign(self, signer: &sr25519::Keypair) -> Self;
	fn set_nonces(self) -> Self;
}

impl CompactTransitionExt for Transition<Compact> {
	fn sign(mut self, signer: &sr25519::Keypair) -> Self {
		sign_with_sr25519!(&mut self, signer);
		self
	}

	fn set_nonces(mut self) -> Self {
		set_nonces!(&mut self, |x| x);
		self
	}
}

/// Utilities that apply to expanded transitions.
pub trait ExpandedTransitionExt
where
	Self: Sized,
{
	/// Converts an expanded transition to a compact transition.
	fn compact(self) -> Transition<Compact>;

	fn sign(self, signer: &sr25519::Keypair) -> Self;
	fn set_nonces(self) -> Self;
}

impl ExpandedTransitionExt for Transition<Expanded> {
	fn compact(self) -> Transition<Compact> {
		Transition {
			inputs: self.inputs.into_iter().map(|x| x.digest()).collect(),
			ephemerals: self.ephemerals,
			outputs: self.outputs,
		}
	}

	fn sign(mut self, signer: &sr25519::Keypair) -> Self {
		sign_with_sr25519!(&mut self, signer);
		self
	}

	fn set_nonces(mut self) -> Self {
		set_nonces!(&mut self, |x: &Object| x.digest());
		self
	}
}
