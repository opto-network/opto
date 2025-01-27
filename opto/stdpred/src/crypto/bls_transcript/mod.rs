use {
	crate::utils::*,
	alloc::vec::Vec,
	blstrs::{Bls12, G1Affine, G1Projective, G2Affine, G2Projective, Gt},
	group::Group,
	opto_core::*,
	opto_onchain_macros::predicate,
	pairing::{MillerLoopResult, MultiMillerLoop},
	scale::{Decode, Encode},
};

mod builder;
mod keys;

#[cfg(test)]
mod tests;

pub use {builder::*, keys::*};

#[predicate(id = 203, core_crate = opto_core)]
pub fn bls_transcript(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	if is_policy(&ctx) {
		// when this predicate is used as a policy then it is a solution
		// to an unlock predicate. The unlock predicate will take care
		// of validating this object.
		ensure!(is_ephemeral(&ctx));
		ensure!(is_only_policy_of_this_type(&ctx));
		ensure!(params.len() == Digest::SIZE);
		ensure!(!ctx.object.data.is_empty());
		true
	} else {
		// as an unlock predicate this policy will expect an ephemeral object
		// with a solution to the expected transcript. The solution to this unlock
		// should be placed in an object with the following config:
		// - It's policy should be `bls_transcript` predicate.
		// - The policy's param should be the hash of the unlock params
		// - The data of the object should be a SCALE serialized `Trasncript` with a
		//   solution.

		let Ok(expectation) = ExpectedTranscript::decode(&mut &params[..]) else {
			return false;
		};

		// ensure all signers are within the expected signers
		ensure!(validate_signers(&expectation));

		// locate a solution to the expected transcript
		let needle = expectation.digest();
		let query = ObjectsSetPattern::fuzzy().must_include(
			ObjectPattern::default()
				.policies(ctx.predicate_id().with_params(move |d: Digest| d == needle))
				.data(DataPattern::named("transcript", |_: Transcript| true)),
		);

		let Some(transcript) = query
			.captures(&transition.ephemerals)
			.into_iter()
			.next()
			.and_then(|c| c.get::<Transcript>("transcript"))
		else {
			return false;
		};

		// we have all expected script entries
		ensure!(transcript.script.len() == expectation.script.len());

		//  hash messages
		let messages = transcript
			.script
			.iter()
			.map(|msg| hash_to_g2(msg))
			.collect::<Vec<_>>();

		// compute the aggregated public key for each script entry
		// by summing up all the public keys of its signers
		let pubkeys = aggregate_pubkeys(&expectation);

		// verify the pairing / signature
		ensure!(verify_pairings(&pubkeys, &messages, transcript.signature.0));

		// signature is valid, now check if all expected data patterns are met
		for (index, entry) in expectation.script.into_iter().enumerate() {
			for condition in entry.expectations {
				if !condition.matches(&transcript.script[index]) {
					return false;
				}
			}
		}

		true // all good
	}
}

pub type SignerIndex = u16;

#[derive(Debug, Clone, Encode, Decode)]
pub struct ScriptEntry {
	/// All the public keys that must sign the entry
	pub signers: Vec<SignerIndex>,

	/// All the conditions that must be met for an entry
	pub expectations: Vec<DataPattern<Cold>>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ExpectedTranscript {
	pub signers: Vec<PublicKey>,
	pub script: Vec<ScriptEntry>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Transcript {
	/// A list of transcript messages that must match the expected transcript
	pub script: Vec<Vec<u8>>,

	/// The aggregated signature of all the signers over all transcript entries
	pub signature: Signature,
}

pub(crate) fn hash_to_g2(message: &[u8]) -> G2Projective {
	const DOMAIN_SEPARATOR: &[u8] =
		b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
	G2Projective::hash_to_curve(message, DOMAIN_SEPARATOR, &[])
}

fn validate_signers(expectation: &ExpectedTranscript) -> bool {
	for entry in &expectation.script {
		for index in &entry.signers {
			if *index as usize >= expectation.signers.len() {
				return false;
			}
		}
	}
	true
}

fn aggregate_pubkeys(expectation: &ExpectedTranscript) -> Vec<G1Projective> {
	use core::ops::Add;
	let mut output = Vec::with_capacity(expectation.script.len());

	for entry in &expectation.script {
		let pubkey = entry
			.signers
			.iter()
			.map(|index| expectation.signers[*index as usize].0)
			.fold(G1Projective::identity(), Add::add);
		output.push(pubkey);
	}

	output
}

fn verify_pairings(
	pubkeys: &[G1Projective],
	messages: &[G2Projective],
	signature: G2Projective,
) -> bool {
	let mut pairings = Gt::identity();
	for (pk, msg) in pubkeys.iter().zip(messages.iter()) {
		pairings += blstrs::pairing(&G1Affine::from(pk), &G2Affine::from(msg));
	}
	let pairing2 = Bls12::multi_miller_loop(&[(
		&G1Projective::generator().into(),
		&G2Affine::from(signature).into(),
	)])
	.final_exponentiation();

	pairings == pairing2
}
