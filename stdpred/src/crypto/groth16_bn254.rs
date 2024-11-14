use {
	crate::{ensure, utils::is_ephemeral},
	alloc::vec::Vec,
	ark_bn254::{Bn254, Fr},
	ark_groth16::{Groth16, Proof},
	ark_serialize::CanonicalDeserialize,
	opto::{
		eval::{Context, Role},
		predicate::AtRest,
		Digest,
		Hashable,
		Transition,
	},
	scale::{Decode, Encode},
};

#[derive(Debug, Clone, Decode, Encode)]
pub enum VerifyingKey {
	Inline(Vec<u8>),
	Ref(Digest),
}

#[derive(Debug, Clone, Decode, Encode)]
pub struct Challenge {
	pub verifying_key: VerifyingKey,
	pub public_inputs: Vec<u8>,
}

#[opto::predicate(id = 203)]
pub fn groth16_bn254(
	ctx: Context<'_>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	match ctx.role {
		Role::Policy(_, _) => {
			ensure!(is_ephemeral(&ctx));
			ensure!(params.len() == 32); // hash of the challenge predicate
			ensure!(ctx.object.data.len() == 128); // https://xn--2-umb.com/23/bn254-compression
			true
		}
		Role::Unlock(predicate, _) => {
			// When used as an unlock predicate this predicate should have its params
			// set to scale encoded `Challenge`` struct that configures the
			// parameters of the expected proof.
			//
			// This predicate will look for any ephemeral object that has a policy
			// of the same type and param that matches the hash of the unlock
			// predicate.
			//
			// it will read the proof bytes from the bytes of the ephemeral object
			// and then verify the proof against the public inputs and the verifying
			// key that is either inlined in the params or referenced object.

			let Ok(challenge) = Challenge::decode(&mut &params[..]) else {
				return false;
			};

			let Ok((vkey, inputs)) = challenge.unpack(transition) else {
				return false;
			};

			let challenge_digest = predicate.digest();
			let proof_policy = AtRest {
				id: ctx.predicate_id(),
				params: challenge_digest.as_ref().to_vec(),
			};

			let Some(proof_obj) = transition
				.ephemerals
				.iter()
				.find(|obj| obj.policies.contains(&proof_policy))
			else {
				return false;
			};

			let Ok(proof) = Proof::deserialize_compressed(proof_obj.data.as_slice())
			else {
				return false;
			};

			let prepared_vk = ark_groth16::prepare_verifying_key(&vkey);

			matches!(
				Groth16::<Bn254>::verify_proof(&prepared_vk, &proof, &inputs),
				Ok(true)
			)
		}
	}
}

impl VerifyingKey {
	#[allow(clippy::result_unit_err)]
	pub fn into_key(
		self,
		transition: &Transition,
	) -> Result<ark_groth16::VerifyingKey<ark_bn254::Bn254>, ()> {
		match self {
			VerifyingKey::Inline(key) => {
				ark_groth16::VerifyingKey::deserialize_compressed(&key[..])
					.map_err(|_| ())
			}
			VerifyingKey::Ref(digest) => {
				let object = transition
					.inputs
					.iter()
					.chain(transition.ephemerals.iter())
					.find(|obj| obj.digest() == digest)
					.ok_or(())?;

				ark_groth16::VerifyingKey::deserialize_compressed(
					object.data.as_slice(),
				)
				.map_err(|_| ())
			}
		}
	}
}

impl Challenge {
	#[allow(clippy::result_unit_err)]
	pub fn unpack(
		self,
		transition: &Transition,
	) -> Result<(ark_groth16::VerifyingKey<ark_bn254::Bn254>, Vec<Fr>), ()> {
		let vk = self.verifying_key.into_key(transition)?;
		let inputs =
			Vec::<Fr>::deserialize_compressed(self.public_inputs.as_slice())
				.map_err(|_| ())?;

		Ok((vk, inputs))
	}
}

#[cfg(test)]
pub mod test {
	use {
		super::*,
		crate::native_impl_factory,
		hex_literal::hex,
		opto::{eval::Location, test::*, transition::Error, Object, PredicateId},
		scale::Encode,
	};

	pub const SP1_V3_0_0_RC4_VK_COMPRESSED: &[u8] = &hex!(
		"3816a8935f51f24e1dd0bce146f3be41468b2ca9c1d407b0d62 \
		 e1c1b9b03861bd02d25e358f8da983ae30c8488fcfee33cef92 \
		 91d0b2f151973f513bef3c720c07b765a3b82d7d707fa350efb \
		 790edb2837bd174156c44661febf696c05457897e2a7c425453 \
		 981c33b5a7dde4446d57b88450684ac11ad228ddc622dfbf092 \
		 88ebf0000e608136da8e177e24d7e36c2bc274f0bd8f2ae9e2d \
		 271acd1f6c459f17660d3669e086e4ee58ac43795a7d639f59a \
		 70c480c0d8a7a15b7adaa782909f5a0f80dc894377ec9308a86 \
		 09e5052cef0bb56611f162d590bc00220bc47e8603000000000 \
		 00000ff7c0a122850c8648c5a451db47b2000d853fb9c3f9888 \
		 340c2e6f82a51cf1174ed67d35d822a81ee122bc6cfcd027da0 \
		 d27a8dc55bbd3271f32fa130f612090d596fed6aa83924c4b96 \
		 3963e4669aecb9d318780cdbdb2dea28ededf57be092"
	);

	pub const SP1_V3_0_0_RC4_VK: &[u8] = &hex!(
		"9b86039b1b1c2ed6b007d4c1a92c8b4641bef346e1bcd01d4ef2515f93a8 \
		 16389e632fd33b25d5283eb70605d292369220e53e91781cf73229bad7b7 \
		 44c50f96c95754c096f6eb1f66446c1574d17b83b2ed90b7ef50a37f707d \
		 2db8a365b7070c723cef3b513f9751f1b2d09192ef3ce3fefc88840ce33a \
		 98daf858e3252dd0df456c1fcd1a272d9eaef2d80b4f27bcc2367e4de277 \
		 e1a86d1308e60000bf8e2809bfdf22c6dd28d21ac14a685084b8576d44e4 \
		 dda7b5331c985354427c2a7eddca83ab0ac98d4315c534fd159e6f27e14b \
		 0dd05b8d446ed1edb02347b245cec67ec40b2200bc90d562f11166b50bef \
		 2c05e509868a30c97e3794c80df8a0f5092978aaadb7157a8a0d0c480ca7 \
		 599f637d5a7943ac58eee486e069360d66170000000397f11ca5826f2e0c \
		 3488983f9cfb53d800207bb41d455a8c64c85028120a7cffd020610f13fa \
		 321f27d3bb55dca8270dda27d0fc6cbc22e11ea822d8357dd64ed2e07bf5 \
		 eded28ea2ddbdb0c7818d3b9ec9a66e46339964b4c9283aad6fe96d50000000000000000"
	);

	const FIB20_PROOF: &[u8] = &hex!(
		"e9bc2fdde7c86689b8a80e9eee744ad9cd3231a95683de7a \
		 9f043e061794c90954115c22379aa9d63989319c9539c562 \
		 d805ae34361f2e3ea175929098ae440df2317ea29ca1f3fd \
		 e73b7f9510f6109128d18ebcc52e6ddab48207b4f257c30c \
		 a7e38f44b2854bc3207a1364a8ff5f92875b684af93a9d2972c972c49e6f008e"
	);
	const FIB20_INPUTS: &[u8] = &hex!(
		"02000000000000007294212c802abadf78938716c39f6859b \
		 8e44b2fe2a4606d50487a10488eaa00909d7ee3f60d938297 \
		 73af47c97bd2eaf35dec0b74c334799ce431cfdeb71c0f"
	);

	#[test]
	fn fib20_vk_inline() {
		let challenge = Challenge {
			verifying_key: VerifyingKey::Inline(
				SP1_V3_0_0_RC4_VK_COMPRESSED.to_vec(),
			),
			public_inputs: FIB20_INPUTS.to_vec(),
		};

		let challenge_pred = AtRest {
			id: PredicateId(203),
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
			vec![predicate(203, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			FIB20_PROOF.to_vec(),
		);

		let transition = Transition {
			inputs: vec![input],
			ephemerals: vec![ephemeral],
			outputs: vec![output1, output2],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}

	#[test]
	fn fib17_vk_inline() {
		const FIB17_PROOF: &[u8] = &hex!(
			"4da7224b33c5fa4588c649006686827ab783b8ea78ab1afad \
			 3a0b825f7d9d71ebaf64ba1ccf391c191de1351b148ec30966 \
			 fedec6d6b155122a5eaa29a83e41a5c122b1441d7dcd5007cc \
			 1bc6ed298845336c3c75010bdf773475a6e8eb4cb1c33299b8 \
			 7123e5e05bc6830806836285ff32a8152139c828e7c653cf40b1bc51a"
		);

		const FIB17_INPUTS: &[u8] = &hex!(
			"02000000000000007294212c802abadf78938716c39f6859b \
			 8e44b2fe2a4606d50487a10488eaa0043ded9117878e861a2 \
			 e8d0980f986878c25556129d631070a220c629bb39581f"
		);

		let challenge = Challenge {
			verifying_key: VerifyingKey::Inline(
				SP1_V3_0_0_RC4_VK_COMPRESSED.to_vec(),
			),
			public_inputs: FIB17_INPUTS.to_vec(),
		};

		let challenge_pred = AtRest {
			id: PredicateId(203),
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
			vec![predicate(203, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			FIB17_PROOF.to_vec(),
		);

		let transition = Transition {
			inputs: vec![input],
			ephemerals: vec![ephemeral],
			outputs: vec![output1, output2],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}

	#[test]
	fn fib20_vk_inline_negative() {
		let challenge = Challenge {
			verifying_key: VerifyingKey::Inline(
				SP1_V3_0_0_RC4_VK_COMPRESSED.to_vec(),
			),
			public_inputs: FIB20_INPUTS.to_vec(),
		};

		let challenge_pred = AtRest {
			id: PredicateId(203),
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
			vec![predicate(203, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			// invalid proof
			FIB20_PROOF.iter().rev().copied().collect::<Vec<_>>(),
		);

		let transition = Transition {
			inputs: vec![input.clone()],
			ephemerals: vec![ephemeral],
			outputs: vec![output1, output2],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(
			evaluation,
			Err(Error::UnlockNotSatisfied(&input, Location::Input))
		);
	}

	#[test]
	fn fib20_vk_ref_input() {
		let circuit_vk_object = Object {
			policies: vec![predicate(100, b"sp1-v3.0.0")],
			unlock: predicate(100, &[1]).into(),
			data: SP1_V3_0_0_RC4_VK_COMPRESSED.to_vec(),
		};

		let challenge = Challenge {
			verifying_key: VerifyingKey::Ref(circuit_vk_object.digest()),
			public_inputs: FIB20_INPUTS.to_vec(),
		};

		// the circuit vk is stored in a referenced object
		// in the real world we will have serveral objects like this
		// that store verification keys for different versions of
		// zk frameworks circuits.
		let challenge_pred = AtRest {
			id: PredicateId(203),
			params: challenge.encode(),
		};

		let challenge_digest = challenge_pred.digest();

		let input1 = opto::test::object(
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
			vec![predicate(203, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			FIB20_PROOF.to_vec(),
		);

		let transition = Transition {
			inputs: vec![input1, circuit_vk_object.clone()],
			ephemerals: vec![ephemeral],
			outputs: vec![output1, output2, circuit_vk_object],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}

	#[test]
	fn fib20_vk_ref_ephemeral() {
		let circuit_vk_object = Object {
			policies: vec![predicate(100, b"sp1-v3.0.0")],
			unlock: predicate(100, &[1]).into(),
			data: SP1_V3_0_0_RC4_VK_COMPRESSED.to_vec(),
		};

		let challenge = Challenge {
			verifying_key: VerifyingKey::Ref(circuit_vk_object.digest()),
			public_inputs: FIB20_INPUTS.to_vec(),
		};

		// the circuit vk is stored in a referenced object
		// in the real world we will have serveral objects like this
		// that store verification keys for different versions of
		// zk frameworks circuits.
		let challenge_pred = AtRest {
			id: PredicateId(203),
			params: challenge.encode(),
		};

		let challenge_digest = challenge_pred.digest();

		let input1 = opto::test::object(
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
			vec![predicate(203, challenge_digest.as_slice())],
			predicate(100, &[1]).into(),
			FIB20_PROOF.to_vec(),
		);

		let transition = Transition {
			inputs: vec![input1],
			ephemerals: vec![ephemeral, circuit_vk_object],
			outputs: vec![output1, output2],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}
}
