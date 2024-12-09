//! Sr25569 signature verification.
//!
//! When a predicate uses this template it can be placed in a policy or unlock.
//!
//! In a policy:
//! - The object must be ephemeral. It must have a signature of the state
//!   transition without ephemeral objects in its data field and the public key
//!   in the params field. This is where the signature is verified.
//! - There must be only one such object in the state transition.
//!
//! In an unlock:
//! - The object will carry only the public key in the params filed.
//! - The unlock will check if there is a corresponding ephemeral object with
//!   the same public key and a signature of the state transition without
//!   ephemeral objects.
//!
//! Sr25519 is the default signature scheme used in Substrate. It is a Schnorr
//! signature scheme with a 256-bit security level. It is used in Substrate for
//! all on-chain signatures. It is the default unlock conditino used when
//! wrapping and unwrapping assets into objects.

use {
	crate::utils::is_ephemeral,
	opto_core::*,
	opto_onchain::predicate,
	schnorrkel::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH},
};

#[predicate(id = 201, core_crate = opto_core)]
pub fn sr25519(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	param: &[u8],
) -> bool {
	let this_predicate_id = ctx.predicate_id();
	match ctx.role {
		// When used as a policy, it describes an object that is a response
		// to a signature challenge. In this case the object must be ephemeral
		// and it's data must be a signature of hashes of all input and output
		// objects.
		Role::Policy(_, _) => {
			ensure!(is_ephemeral(&ctx));
			ensure!(param.len() == PUBLIC_KEY_LENGTH);
			ensure!(ctx.object.data.len() == SIGNATURE_LENGTH);

			let Ok(pubkey) = PublicKey::from_bytes(param) else {
				return false;
			};

			let Ok(sig) = Signature::from_bytes(&ctx.object.data) else {
				return false;
			};

			// as a response to a challenge, ensure that there is only one
			// such response for the given public key in this state transition.
			let challenge_responses = transition.ephemerals.iter().filter(|obj| {
				obj.policies.iter().any(|policy| {
					policy.id == this_predicate_id && policy.params == param
				})
			});

			ensure!(challenge_responses.count() == 1);

			// actual signature validation happens on the ephemeral object only and
			// unlock only checks if there is a corresponding ephemeral object. The
			// reason for this is to reduce the computation cost of signature
			// verification, where we should have only one ephemeral object with a
			// signature for a given public key and we can have many objects that
			// are unlocked by that signature.

			// calculate the hash of all inputs and outputs without ephemeral objects
			let message = transition.digest_for_signing();

			// the context for the signature. This is the value used across all
			// substrate chains.
			const SIGNING_CTX: &[u8] = b"substrate";

			// verify the signature
			ensure!(pubkey
				.verify_simple(SIGNING_CTX, message.as_slice(), &sig)
				.is_ok());
		}

		// when used as an unlock, it gates the consumption of an input object
		// by requiring a corresponding ephemeral object with a signature of
		// hashes of all input and output objects.
		Role::Unlock(_, _) => {
			let challenge_response = transition.ephemerals.iter().find(|obj| {
				obj.policies.iter().any(|policy| {
					policy.id == this_predicate_id && policy.params == param
				})
			});

			// make sure that there is an ephemeral that is a response to the
			// challenge for the given public key. If there is one, then we are
			// good to go. The actual validation happens on the ephemeral object
			// that is a response to the challenge to avoid duplicated work.
			ensure!(challenge_response.is_some());
		}
	}

	true
}

#[cfg(feature = "offchain")]
pub trait TransitionExt
where
	Self: Sized,
{
	type Error;

	fn sign_with_sr25519(&mut self, signer: &Keypair);
}

#[cfg(feature = "offchain")]
use subxt_signer::sr25519::Keypair;

#[cfg(feature = "offchain")]
impl TransitionExt for Transition<Compact> {
	type Error = schnorrkel::SignatureError;

	fn sign_with_sr25519(&mut self, signer: &Keypair) {
		let predicate_id = sr25519_id;
		let pubkey = signer.public_key();
		let predicate = opto_core::Predicate {
			id: predicate_id,
			params: pubkey.as_ref().to_vec(),
		};

		// first check if we already have a signature for this signer
		if self
			.ephemerals
			.iter()
			.any(|obj| obj.policies.iter().any(|p| p == &predicate))
		{
			return; // already signed
		}

		// if not, then add a new ephemeral object with the signature
		let signature_object = Object {
			policies: alloc::vec![predicate],
			unlock: Predicate {
				id: crate::util::constant::constant_id,
				params: [1].to_vec(),
			}
			.into(),
			data: signer
				.sign(self.digest_for_signing().as_slice())
				.as_ref()
				.to_vec(),
		};

		self.ephemerals.push(signature_object);
	}
}
