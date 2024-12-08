use {
	super::signature::{signature_verification, Verifier},
	opto_core::*,
	opto_onchain::predicate,
	schnorrkel::{PublicKey, Signature, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH},
};

const SIGNING_CTX: &[u8] = b"substrate";

struct Sr25519SubstrateVerifier;
impl Verifier for Sr25519SubstrateVerifier {
	const PUBLIC_KEY_LENGTH: usize = PUBLIC_KEY_LENGTH;
	const SIGNATURE_LENGTH: usize = SIGNATURE_LENGTH;

	fn verify(
		&self,
		public_key: &[u8],
		message: &[u8],
		signature: &[u8],
	) -> bool {
		let Ok(pubkey) = PublicKey::from_bytes(public_key) else {
			return false;
		};

		let Ok(sig) = Signature::from_bytes(signature) else {
			return false;
		};

		pubkey.verify_simple(SIGNING_CTX, message, &sig).is_ok()
	}
}

#[predicate(id = 201, core_crate = opto_core)]
pub fn sr25519(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	param: &[u8],
) -> bool {
	signature_verification(ctx, transition, param, Sr25519SubstrateVerifier)
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
