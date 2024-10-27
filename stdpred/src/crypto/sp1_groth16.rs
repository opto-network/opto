//! SP1 Groth16 verifier predicate
//!
//! Verifies proofs generated by SP1 using it's conventions.
//! In part inspired by https://github.com/succinctlabs/sp1-solana

use {
	crate::{ensure, utils::is_ephemeral},
	alloc::vec::Vec,
	ark_bn254::{Bn254, Fr, G1Affine, G2Affine},
	ark_ec::AffineRepr,
	ark_groth16::{Groth16, PreparedVerifyingKey, Proof, VerifyingKey},
	ark_serialize::{CanonicalDeserialize, Compress, Validate},
	cid::Cid,
	core::array::TryFromSliceError,
	num_bigint::BigUint,
	opto::{repr::AtRest, Context, Hashable, PredicateId, Role, Transition},
	scale::{Decode, Encode},
	sha2::{Digest, Sha256},
};

#[derive(Debug, Clone, Decode, Encode)]
pub struct Sp1Groth16Challenge {
	pub app_key: [u8; 32],
	pub public_values: Vec<u8>,
	pub code_cid: Option<Cid>,
}

#[opto::predicate(id = 204)]
pub fn sp1_groth16(
	ctx: Context<'_>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	match ctx.role {
		Role::Policy(_, _) => {
			ensure!(is_ephemeral(&ctx));
			ensure!(params.len() == 32); // hash of the unlock predicate
			ensure!(ctx.object.data.len() == 260); // 4 bytes sp1groth vk hash + 256 bytes proof
			true
		}
		Role::Unlock(me, _) => transition.ephemerals.iter().any(|_| {
			// When used as an unlock predicate this predicate should have its params
			// to the outputs of an SP1 proof running groth16 prover. To see an
			// example of how to generate those values see
			// https://github.com/succinctlabs/sp1-project-template/tree/main?tab=readme-ov-file#generate-an-evm-compatible-proof
			//
			// more specifically get the sp1-project-template and run the following:
			// ```
			// cd script
			// cargo run --release --bin evm -- --system groth16
			// ```
			// as if you were generating a proof for verification on the EVM.
			//
			// circuit verification keys: https://github.com/succinctlabs/sp1-solana/tree/master/verifier/vk
			//
			// This predicate will look for any ephemeral object that has a policy
			// of the same type and param that matches the hash of the unlock
			// predicate.
			//
			// it will read the proof bytes from the bytes of the ephemeral object
			// and then verify the proof against the public inputs and the verifying
			// key that is either inlined in the params or referenced object.

			let Ok(challenge) = Sp1Groth16Challenge::decode(&mut &params[..]) else {
				return false;
			};

			// convert public commited values to circuit public inputs
			let Ok((appkey, commited_vals)) =
				groth16_public_values(&challenge.app_key, &challenge.public_values)
			else {
				return false;
			};

			let public_inputs = [Fr::from(appkey), Fr::from(commited_vals)];

			// find an ephemeral object that has the same policy as the unlock
			// and its param is equal to the hash of this unlock predicate
			let Some(proof_bytes) = find_challenge_response(&ctx, transition, me)
			else {
				// no proof was found as part of this state transistion that
				// matches the unlock predicate
				return false;
			};

			// 4 bytes for groth16 vk hash prefix + 256 bytes for the proof
			ensure!(proof_bytes.len() == 260);
			let hash_prefix = &proof_bytes[..4];

			let Ok(proof_bytes): Result<[u8; 256], _> = proof_bytes[4..].try_into()
			else {
				return false;
			};

			let Ok(proof) = deserialize_proof(&proof_bytes) else {
				return false;
			};

			let Some(circuit_vk) = find_sp1_circuit_vk(hash_prefix, transition)
			else {
				// no circuit vk was found as part of this state transition,
				return false;
			};

			let Ok(vk) = deserialize_vk(circuit_vk) else {
				return false;
			};

			let pvk: PreparedVerifyingKey<Bn254> = vk.into();

			matches!(
				Groth16::<Bn254>::verify_proof(&pvk, &proof, &public_inputs[..]),
				Ok(true)
			)
		}),
	}
}

fn find_challenge_response<'a>(
	ctx: &Context<'_>,
	transition: &'a Transition,
	challenge_predicate: &AtRest,
) -> Option<&'a [u8]> {
	let challenge_digest = challenge_predicate.digest();
	let needle = challenge_digest.as_slice();
	transition.ephemerals.iter().find_map(|obj| {
		if obj.policies.iter().any(|policy| {
			policy.id == ctx.predicate_id() && policy.params.as_slice() == needle
		}) {
			Some(obj.data.as_slice())
		} else {
			None
		}
	})
}

/// Looks in input and ephemeral objects for an `const` with param starting with
/// `sp1`. If it's body hashes to a hash that startes with `hash_prefix` then
/// it is the groth16 verifier key for the cercuit of the SP1 runtime used to
/// compile the program.
fn find_sp1_circuit_vk<'a>(
	hash_prefix: &[u8],
	transition: &'a Transition,
) -> Option<&'a [u8]> {
	transition
		.inputs
		.iter()
		.chain(transition.ephemerals.iter())
		.find(|obj| {
			obj.policies.iter().any(|policy| {
				policy.id == PredicateId(100) && policy.params.starts_with(b"sp1")
			}) && Sha256::digest(obj.data.as_slice()).starts_with(hash_prefix)
		})
		.map(|obj| obj.data.as_slice())
}

/// SP1 has 2 public inputs to the verifier:
/// 1. The hash of the app riscv elf executable that is being verified (appkey),
/// 32 bytes, specific to the SP1 program.
/// 2. The hash of the public inputs to the SP1 program elf.
fn groth16_public_values(
	sp1_vkey_hash: &[u8; 32],
	sp1_public_inputs: &[u8],
) -> Result<(BigUint, BigUint), ()> {
	let mut committed_values_hash: [u8; 32] =
		Sha256::digest(sp1_public_inputs).into();
	// The Groth16 verifier operates over a 254 bit field (BN254), so we need to
	// zero out the first 3 bits. The same logic happens in the SP1 Ethereum
	// verifier contract.
	committed_values_hash[0] &= 0b00011111;

	let commited_values_public_input =
		BigUint::from_bytes_be(&committed_values_hash);

	let appkey = BigUint::from_bytes_be(&sp1_vkey_hash[1..]);

	Ok((appkey, commited_values_public_input))
}

fn deserialize_proof(bytes: &[u8; 256]) -> Result<Proof<Bn254>, InternalError> {
	let a: G1 =
		PodG1(endianness_64(&bytes[..64]).as_slice().try_into()?).try_into()?;

	let b: G2 =
		PodG2(endianness_128(&bytes[64..192]).as_slice().try_into()?).try_into()?;

	let c: G1 =
		PodG1(endianness_64(&bytes[192..256]).as_slice().try_into()?).try_into()?;

	Ok(Proof { a, b, c })
}

/// deserialize the sp1 circuit verifying key from bytes.
/// Those circuits are most often stored in const objects and reused across all
/// zk proofs generated by the SP1 runtime.
fn deserialize_vk(bytes: &[u8]) -> Result<VerifyingKey<Bn254>, InternalError> {
	let alpha_g1 = bytes_to_g1(&bytes[..32].try_into()?)?;
	let beta_g2 = bytes_to_g2(&bytes[64..128].try_into()?)?;
	let gamma_g2 = bytes_to_g2(&bytes[128..192].try_into()?)?;
	let delta_g2 = bytes_to_g2(&bytes[224..288].try_into()?)?;

	if bytes.len() < 292 {
		return Err(InternalError::SliceOutOfBounds);
	}

	let num_k =
		u32::from_be_bytes([bytes[288], bytes[289], bytes[290], bytes[291]]);
	let mut gamma_abc_g1 = Vec::with_capacity(num_k as usize);
	let mut offset = 292;

	for _ in 0..num_k {
		if bytes.len() < offset + 31 {
			return Err(InternalError::SliceOutOfBounds);
		}

		let point = bytes_to_g1(bytes[offset..offset + 32].try_into()?)?;
		gamma_abc_g1.push(point);
		offset += 32;
	}

	Ok(VerifyingKey {
		alpha_g1,
		beta_g2,
		gamma_g2,
		delta_g2,
		gamma_abc_g1,
	})
}

type G1 = ark_bn254::g1::G1Affine;
type G2 = ark_bn254::g2::G2Affine;

#[derive(Debug)]
pub enum InternalError {
	InvalidInputData,
	GroupError,
	SliceOutOfBounds,
	UnexpectedError,
	TryIntoVecError(Vec<u8>),
	ProjectiveToG1Failed,
}

impl From<TryFromSliceError> for InternalError {
	fn from(_: TryFromSliceError) -> Self {
		InternalError::SliceOutOfBounds
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct PodG1(pub [u8; 64]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct PodG2(pub [u8; 128]);

impl TryFrom<PodG1> for G1 {
	type Error = InternalError;

	fn try_from(bytes: PodG1) -> Result<Self, InternalError> {
		if bytes.0 == [0u8; 64] {
			return Ok(G1::zero());
		}
		let g1 = Self::deserialize_with_mode(
			&*[&bytes.0[..], &[0u8][..]].concat(),
			Compress::No,
			Validate::Yes,
		);

		match g1 {
			Ok(g1) => {
				if !g1.is_on_curve() {
					Err(InternalError::GroupError)
				} else {
					Ok(g1)
				}
			}
			Err(_) => Err(InternalError::InvalidInputData),
		}
	}
}

impl TryFrom<PodG2> for G2 {
	type Error = InternalError;

	fn try_from(bytes: PodG2) -> Result<Self, Self::Error> {
		if bytes.0 == [0u8; 128] {
			return Ok(G2::zero());
		}
		let g2 = Self::deserialize_with_mode(
			&*[&bytes.0[..], &[0u8][..]].concat(),
			Compress::No,
			Validate::Yes,
		);

		match g2 {
			Ok(g2) => {
				if !g2.is_on_curve() {
					Err(InternalError::GroupError)
				} else {
					Ok(g2)
				}
			}
			Err(_) => Err(InternalError::InvalidInputData),
		}
	}
}

fn endianness_64(bytes: &[u8]) -> Vec<u8> {
	bytes
		.chunks(32)
		.flat_map(|b| b.iter().copied().rev().collect::<Vec<u8>>())
		.collect::<Vec<u8>>()
}

fn endianness_128(bytes: &[u8]) -> Vec<u8> {
	bytes
		.chunks(64)
		.flat_map(|b| b.iter().copied().rev().collect::<Vec<u8>>())
		.collect::<Vec<u8>>()
}

const GNARK_MASK: u8 = 0b11 << 6;
const GNARK_COMPRESSED_POSTIVE: u8 = 0b10 << 6;
const GNARK_COMPRESSED_NEGATIVE: u8 = 0b11 << 6;
const GNARK_COMPRESSED_INFINITY: u8 = 0b01 << 6;

const ARK_MASK: u8 = 0b11 << 6;
const ARK_COMPRESSED_POSTIVE: u8 = 0b00 << 6;
const ARK_COMPRESSED_NEGATIVE: u8 = 0b10 << 6;
const ARK_COMPRESSED_INFINITY: u8 = 0b01 << 6;

fn gnark_flag_to_ark_flag(msb: u8) -> u8 {
	let gnark_flag = msb & GNARK_MASK;

	let ark_flag = match gnark_flag {
		GNARK_COMPRESSED_POSTIVE => ARK_COMPRESSED_POSTIVE,
		GNARK_COMPRESSED_NEGATIVE => ARK_COMPRESSED_NEGATIVE,
		GNARK_COMPRESSED_INFINITY => ARK_COMPRESSED_INFINITY,
		_ => panic!("Invalid GNARK flag: {:b}", gnark_flag),
	};

	msb & !ARK_MASK | ark_flag
}

fn gnark_compressed_x_to_ark_compressed_x(
	x: &[u8],
) -> Result<Vec<u8>, InternalError> {
	if x.len() != 32 && x.len() != 64 {
		return Err(InternalError::InvalidInputData);
	}

	let mut x_copy = x.to_vec();
	let msb = gnark_flag_to_ark_flag(x_copy[0]);
	x_copy[0] = msb;

	x_copy.reverse();
	Ok(x_copy)
}

fn bytes_to_g1(g1_bytes: &[u8; 32]) -> Result<G1Affine, InternalError> {
	let g1_bytes = gnark_compressed_x_to_ark_compressed_x(g1_bytes)?;
	G1Affine::deserialize_with_mode(&g1_bytes[..], Compress::Yes, Validate::No)
		.map_err(|_| InternalError::GroupError)
}

fn bytes_to_g2(g2_bytes: &[u8; 64]) -> Result<G2Affine, InternalError> {
	let g2_bytes = gnark_compressed_x_to_ark_compressed_x(g2_bytes)?;
	G2Affine::deserialize_with_mode(&g2_bytes[..], Compress::Yes, Validate::No)
		.map_err(|_| InternalError::GroupError)
}

#[cfg(test)]
mod test {
	use {
		super::*,
		crate::{
			crypto::groth16_bn254::test::SP1_V3_0_0_RC4_VK,
			native_impl_factory,
		},
		hex_literal::hex,
		opto::{test::predicate, Object},
	};

	#[test]
	fn fib20_smoke() {
		let sp1_vkey_hash =
			hex!("00aa8e48107a48506d60a4e22f4be4b859689fc316879378dfba2a802c219472");
		let sp1_public_inputs = hex!("0000000000000000000000000000000000000000000000000000000000000011000000000000000000000000000000000000000000000000000000000000063d0000000000000000000000000000000000000000000000000000000000000a18");
		let sp1_proof = hex!("feb5e54e1ed7d9f725b8a0d3fa1aab78eab883b77a8286660049c68845fac5334b22a74d032e5e4458481c19ef78c1b05632235003fd515d6ae1e9f7527ba9791bfbfe401ccbb48e6e5a4773f7bd1050c7c336538498d26ebcc17c00d5dcd741142b125c1ae4839aa2eaa52251156b6deced6f9630ec48b15113de91c191f3cca14bf6ba0b49343eee9ab54a213374eb384fd5d776c11f6042102b1bbb929ba7d76f605e0d950bd635554c1c0ab6f8c7d333ef31412f7970439fdbcdae6de42124e7a84d1ac51b0bf43c657c8e829c1352812af35f283668803068bc055e3e12879b293302a0c09d2598e33007d2210f56ed7637e9a3574af91ee068e1f2acf04ebe196c");

		// a const object that stores the sp1 v3.0.0 groth16 circuit vk
		let sp1_v3_vk_obj = Object {
			policies: vec![predicate(100, b"sp1-v3.0.0")],
			unlock: predicate(100, &[1]).into(),
			data: SP1_V3_0_0_RC4_VK.to_vec(),
		};

		let challenge = Sp1Groth16Challenge {
			app_key: sp1_vkey_hash,
			public_values: sp1_public_inputs.to_vec(),
			code_cid: None,
		};

		let challenge_pred = AtRest {
			id: PredicateId(204),
			params: challenge.encode(),
		};

		let challenge_digest = challenge_pred.digest();

		let input = opto::test::object(
			vec![predicate(1000, b"USDT")],
			challenge_pred.into(),
			1000u64.to_le_bytes().to_vec(),
		);

		let output1 = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
			500u64.to_le_bytes().to_vec(),
		);

		let output2 = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
			500u64.to_le_bytes().to_vec(),
		);

		let ephemeral = opto::test::object(
			vec![predicate(204, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			sp1_proof.to_vec(),
		);

		let transition = Transition {
			inputs: vec![sp1_v3_vk_obj.clone(), input],
			ephemerals: vec![ephemeral],
			outputs: vec![output1, output2, sp1_v3_vk_obj],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}
}
