use {
	super::signature::{signature_verification, Verifier},
	opto_core::{
		eval::Context,
		repr::Compact,
		signer::sr25519::Keypair,
		AtRest,
		Object,
		Transition,
	},
	schnorrkel::{
		PublicKey,
		Signature,
		SignatureError,
		PUBLIC_KEY_LENGTH,
		SIGNATURE_LENGTH,
	},
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

#[opto_onchain::predicate(id = 201, core_crate = opto_core)]
pub fn sr25519(
	ctx: Context<'_>,
	transition: &Transition,
	param: &[u8],
) -> bool {
	signature_verification(ctx, transition, param, Sr25519SubstrateVerifier)
}

pub trait TransitionExt
where
	Self: Sized,
{
	type Error;

	fn sign_sr25519(&mut self, signer: &Keypair);
}

impl TransitionExt for Transition<Compact> {
	type Error = SignatureError;

	fn sign_sr25519(&mut self, signer: &Keypair) {
		let predicate_id = sr25519_id;
		let pubkey = signer.public_key();
		let predicate = opto_core::AtRest {
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
			unlock: AtRest {
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
