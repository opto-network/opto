use {
	super::{
		ExpectedTranscript,
		PublicKey,
		SecretKey,
		Signature,
		SignerIndex,
		Transcript,
	},
	alloc::vec::Vec,
	blstrs::{G1Affine, G2Projective},
	group::{prime::PrimeCurveAffine, Group},
	opto_core::{Hashable, Object, PredicateIdExt},
	scale::Encode,
};

#[derive(Debug)]
pub enum Error {
	/// Error that is returned when trying to add a signature
	/// to a transcript entry that does not exist.
	EntryDoesNotExist,

	/// The error is returned when trying to add more entries
	/// than the expected number of entries in the transcript.
	TooManyEntries,

	/// The error is returned when trying to sign an entry
	/// using a public key that is not in the signers list.
	UnexpectedSigner,

	/// The error is returned when trying to sign an entry
	/// more than the expected number of signers.
	TooManySigners,

	/// No signatures added to the transcript.
	MissingSignatures,
}

#[derive(Debug, Clone)]
pub struct TranscriptBuilder<'e> {
	expectation: &'e ExpectedTranscript,

	/// Individual messages
	script: Vec<Vec<u8>>,

	/// Aggregate signature
	signature: Signature,
}

impl<'e> From<&'e ExpectedTranscript> for TranscriptBuilder<'e> {
	fn from(expectation: &'e ExpectedTranscript) -> Self {
		Self {
			expectation,
			script: Vec::new(),
			signature: Signature(G2Projective::identity()),
		}
	}
}

impl TranscriptBuilder<'_> {
	/// Adds a message to the transcript and returns its index.
	pub fn add_entry(&mut self, message: impl Encode) -> Result<usize, Error> {
		let index = self.script.len();

		if index >= self.expectation.script.len() {
			return Err(Error::TooManyEntries);
		}

		self.script.push(message.encode());

		Ok(index)
	}

	/// Given a private key and an index of an entry, signs the entry
	/// and adds the signature to the transcript aggregate signature.
	pub fn sign_entry(
		&mut self,
		index: usize,
		signer: &SecretKey,
	) -> Result<(), Error> {
		if index >= self.script.len() {
			return Err(Error::EntryDoesNotExist);
		}

		let pubkey = G1Affine::generator() * signer.0;

		// check if signer is on the signers list and if it is
		// get the index of the signer in the signers list
		let signer_index = self
			.expectation
			.signers
			.iter()
			.position(|s| s == &PublicKey(pubkey))
			.ok_or(Error::UnexpectedSigner)? as SignerIndex;

		// check if the signer is expected to sign the entry
		if !self.expectation.script[index]
			.signers
			.contains(&signer_index)
		{
			return Err(Error::UnexpectedSigner);
		}

		// sign the entry and add the signature to the transcript. The signature
		// is the secret scalar multiplied by the hash of the message.
		let signature = signer.sign(&self.script[index]);

		// add to the message signatures aggregate
		self.add_signature(signature)?;

		Ok(())
	}

	pub fn add_signature(&mut self, signature: Signature) -> Result<(), Error> {
		// add the signature to the aggregate signature
		self.signature.0 += signature.0;
		Ok(())
	}
}

impl TranscriptBuilder<'_> {
	pub fn build(self) -> Result<Transcript, Error> {
		if self.signature.0.is_identity().into() {
			return Err(Error::MissingSignatures);
		}

		Ok(Transcript {
			// store individual messages without their signatures
			script: self.script,
			// aggregate all signatures into one
			signature: self.signature,
		})
	}

	pub fn build_object(self) -> Result<Object, Error> {
		Ok(Object {
			policies: alloc::vec![
				crate::ids::BLS_TRANSCRIPT.params(self.expectation.digest())
			],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: self.build()?.encode(),
		})
	}
}
