use {
	crate::{ensure, utils::is_ephemeral},
	opto::{Context, Role, Transition},
};

pub trait Verifier {
	const SIGNATURE_LENGTH: usize;
	const PUBLIC_KEY_LENGTH: usize;

	fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8])
		-> bool;
}

/// Generic signature verification template for all types of signatures.
///
/// When a predicate uses this template it can be placed in a policy or unlock.
///
/// In a policy:
/// - The object must be ephemeral. It must have a signature of the state
///   transition without ephemeral objects in its data field and the public key
///   in the params field. This is where the signature is verified.
/// - There must be only one such object in the state transition.
///
/// In an unlock:
/// - The object will carry only the public key in the params filed.
/// - The unlock will check if there is a corresponding ephemeral object with
///   the same public key and a signature of the state transition without
///   ephemeral objects.
///
/// Implementations using this template should pass their paramters as is to
/// this template plus an extra paramters that has a function that can verify
/// the signature.
pub fn signature_verification<V: Verifier>(
	ctx: Context<'_>,
	transition: &Transition,
	param: &[u8],
	verifier: V,
) -> bool {
	let this_predicate_id = ctx.predicate_id();

	match ctx.role {
		// When used as a policy, it describes an object that is a response
		// to a signature challenge. In this case the object must be ephemeral
		// and it's data must be a signature of hashes of all input and output
		// objects.
		Role::Policy(_, _) => {
			ensure!(is_ephemeral(&ctx));
			ensure!(param.len() == V::PUBLIC_KEY_LENGTH);
			ensure!(ctx.object.data.len() == V::SIGNATURE_LENGTH);

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

			// invoke the actual verification algorithm
			ensure!(verifier.verify(param, message.as_slice(), &ctx.object.data));
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
