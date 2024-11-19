use {
	super::signature::{signature_verification, Verifier},
	ed25519_dalek::{
		Signature,
		VerifyingKey,
		PUBLIC_KEY_LENGTH,
		SIGNATURE_LENGTH,
	},
	opto_core::{eval::Context, Transition},
};

struct Ed25519Verifier;
impl Verifier for Ed25519Verifier {
	const PUBLIC_KEY_LENGTH: usize = PUBLIC_KEY_LENGTH;
	const SIGNATURE_LENGTH: usize = SIGNATURE_LENGTH;

	fn verify(
		&self,
		public_key: &[u8],
		message: &[u8],
		signature: &[u8],
	) -> bool {
		let Ok(pubkey): Result<[u8; PUBLIC_KEY_LENGTH], _> = public_key.try_into()
		else {
			return false;
		};

		let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey) else {
			return false;
		};

		let Ok(sig): Result<[u8; SIGNATURE_LENGTH], _> = signature.try_into()
		else {
			return false;
		};

		let sig = Signature::from_bytes(&sig);

		pubkey.verify_strict(message, &sig).is_ok()
	}
}

#[opto_onchain::predicate(id = 200, core_crate = opto_core)]
pub fn ed25519(
	ctx: Context<'_>,
	transition: &Transition,
	param: &[u8],
) -> bool {
	signature_verification(ctx, transition, param, Ed25519Verifier)
}
