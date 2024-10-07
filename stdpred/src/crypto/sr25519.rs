use {
	super::signature::{signature_verification, Verifier},
	opto::{Context, Transition},
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

#[opto::predicate(id = 201)]
pub fn sr25519(
	ctx: Context<'_>,
	transition: &Transition,
	param: &[u8],
) -> bool {
	signature_verification(ctx, transition, param, Sr25519SubstrateVerifier)
}
