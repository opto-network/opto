use {
	super::*,
	alloc::vec,
	opto_core::test::*,
	scale::{Decode, Encode},
};

fn machine() -> MockMachine {
	let mut machine = MockMachine::default();
	machine.add_predicate(crate::ids::BLS_TRANSCRIPT, bls_transcript);
	machine.add_predicate(crate::ids::COIN, crate::asset::coin);
	machine.add_predicate(crate::ids::CONSTANT, crate::util::constant);
	machine
}

#[derive(Clone, Debug, PartialEq, Eq, Encode)]
struct ParamStruct {
	a: u32,
	b: u32,
}

fn expectation(signer1: PublicKey, signer2: PublicKey) -> ExpectedTranscript {
	ExpectedTranscript {
		signers: vec![signer1, signer2],
		script: vec![
			ScriptEntry {
				signers: vec![0, 1],
				expectations: vec![
					DataPattern::<Cold>::new((0..4).equals(85u32)),
					DataPattern::<Cold>::new((4..8).greater_than(95u32)),
				],
			},
			ScriptEntry {
				signers: vec![0],
				expectations: vec![
					DataPattern::<Cold>::new((0..4).equals(185u32)),
					DataPattern::<Cold>::new((4..8).greater_than(195u32)),
				],
			},
			ScriptEntry {
				signers: vec![1],
				expectations: vec![
					DataPattern::<Cold>::new((0..4).equals(285u32)),
					DataPattern::<Cold>::new((4..8).greater_than(395u32)),
				],
			},
		],
	}
}

fn intent(signer1: PublicKey, signer2: PublicKey) -> Object {
	Object {
		policies: vec![crate::ids::COIN.params(1u32)],
		unlock: crate::ids::BLS_TRANSCRIPT
			.params(expectation(signer1, signer2))
			.into(),
		data: 50000u64.encode(),
	}
}

#[test]
fn one_signer_smoke() {
	let signer = SecretKey::generate(&mut rand::thread_rng());

	let expectation = ExpectedTranscript {
		signers: vec![signer.public_key()],
		script: vec![ScriptEntry {
			signers: vec![0],
			expectations: vec![
				DataPattern::<Cold>::new((0..4).equals(85u32)),
				DataPattern::<Cold>::new((4..8).greater_than(95u32)),
			],
		}],
	};

	let intent = Object {
		policies: vec![crate::ids::COIN.params(1u32)],
		unlock: crate::ids::BLS_TRANSCRIPT
			.params(expectation.clone())
			.into(),
		data: 50000u64.encode(),
	};

	let solution = Object {
		policies: vec![crate::ids::BLS_TRANSCRIPT.params(expectation.digest())],
		unlock: crate::ids::CONSTANT.params(1).into(),
		data: Transcript {
			script: vec![ParamStruct { a: 85, b: 98 }.encode()],
			signature: signer.sign(&ParamStruct { a: 85, b: 98 }.encode()),
		}
		.encode(),
	};

	let machine = machine();
	let env = StaticEnvironment::default();

	let transition = Transition::<Expanded> {
		inputs: vec![intent],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000u64.encode(),
		}],
		ephemerals: vec![solution],
	};

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(transition_exec.evaluate(&transition, &env), Ok(()));
}

#[test]
fn smoke_negative_no_solution() {
	let machine = machine();
	let env = StaticEnvironment::default();

	let signer1 = SecretKey::generate(&mut rand::thread_rng());
	let signer2 = SecretKey::generate(&mut rand::thread_rng());

	let transition = Transition::<Expanded> {
		inputs: vec![intent(signer1.public_key(), signer2.public_key())],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000.encode(),
		}],
		ephemerals: vec![],
	};

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(
		transition_exec.evaluate(&transition, &env),
		Err(EvalError::UnlockNotSatisfied(
			&transition.inputs[0],
			Location::Input
		))
	);
}

#[test]
fn smoke_negative_invalid_solution() {
	let machine = machine();
	let env = StaticEnvironment::default();

	let signer1 = SecretKey::generate(&mut rand::thread_rng());
	let signer2 = SecretKey::generate(&mut rand::thread_rng());

	let transition = Transition::<Expanded> {
		inputs: vec![intent(signer1.public_key(), signer2.public_key())],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000.encode(),
		}],
		ephemerals: vec![],
	};

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();

	assert_eq!(
		transition_exec.evaluate(&transition, &env),
		Err(EvalError::UnlockNotSatisfied(
			&transition.inputs[0],
			Location::Input
		))
	);
}

#[test]
fn smoke_positive() {
	let machine = machine();
	let env = StaticEnvironment::default();

	let signer1 = SecretKey::generate(&mut rand::thread_rng());
	let signer2 = SecretKey::generate(&mut rand::thread_rng());

	let challenge = expectation(
		signer1.public_key(), //
		signer2.public_key(),
	);

	let mut solution_builder = TranscriptBuilder::from(&challenge);

	let ix = solution_builder
		.add_entry(ParamStruct { a: 85, b: 98 })
		.unwrap();
	solution_builder.sign_entry(ix, &signer1).unwrap();
	solution_builder.sign_entry(ix, &signer2).unwrap();

	let ix = solution_builder
		.add_entry(ParamStruct { a: 185, b: 198 })
		.unwrap();
	solution_builder.sign_entry(ix, &signer1).unwrap();

	let ix = solution_builder
		.add_entry(ParamStruct { a: 285, b: 398 })
		.unwrap();
	solution_builder.sign_entry(ix, &signer2).unwrap();

	let transition = Transition::<Expanded> {
		inputs: vec![intent(signer1.public_key(), signer2.public_key())],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000u64.encode(),
		}],
		ephemerals: vec![solution_builder.build_object().unwrap()],
	};

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(transition_exec.evaluate(&transition, &env), Ok(()));
}

#[test]
fn quark_pulse() {
	const NONCE: u32 = 1170276544;
	const PUBKEY: [u8; 48] = [
		151, 8, 109, 11, 177, 216, 18, 64, 245, 104, 104, 232, 76, 155, 224, 237,
		93, 98, 143, 142, 193, 66, 98, 116, 58, 188, 94, 226, 115, 24, 19, 243,
		163, 59, 146, 86, 158, 137, 102, 126, 170, 23, 194, 132, 121, 135, 31, 33,
	];

	const MESSAGE: [u8; 36] = [
		12, 135, 143, 103, 40, 88, 12, 0, 27, 34, 0, 0, 5, 0, 0, 0, 192, 0, 193,
		69, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
	];

	const SIGNATURE: [u8; 96] = [
		131, 248, 115, 230, 123, 190, 234, 202, 160, 57, 41, 220, 110, 141, 17,
		193, 254, 145, 161, 74, 236, 170, 213, 151, 130, 123, 60, 111, 195, 170,
		228, 210, 59, 244, 17, 152, 196, 51, 101, 86, 2, 104, 72, 85, 112, 245,
		132, 95, 6, 8, 79, 182, 4, 44, 15, 224, 109, 131, 100, 84, 34, 31, 147,
		225, 75, 70, 77, 23, 148, 45, 253, 16, 66, 164, 159, 134, 27, 49, 53, 19,
		94, 78, 169, 147, 31, 181, 11, 197, 19, 242, 211, 55, 64, 224, 76, 136,
	];

	let transcript = Transcript {
		script: vec![MESSAGE.to_vec()],
		signature: Signature::decode(&mut SIGNATURE.as_slice()).unwrap(),
	};

	let expectation = ExpectedTranscript {
		signers: vec![PublicKey::decode(&mut PUBKEY.as_slice()).unwrap()],
		script: vec![ScriptEntry {
			signers: vec![0],
			expectations: vec![
				// session id
				DataPattern::<Cold>::new((4..8).equals(809000)),
				// role id
				DataPattern::<Cold>::new((8..12).greater_than(8730)),
				// uptime
				DataPattern::<Cold>::new((12..16).greater_than_or_equals(3)),
				DataPattern::<Cold>::new((12..16).less_than_or_equals(6)),
				// nonce
				DataPattern::<Cold>::new((16..20).equals(NONCE)),
				// comulative tx
				DataPattern::<Cold>::new((20..28).equals(3u64)),
				// cumulative rx
				DataPattern::<Cold>::new((28..36).equals(2u64)),
			],
		}],
	};

	let solution = Object {
		policies: vec![crate::ids::BLS_TRANSCRIPT.params(expectation.digest())],
		unlock: crate::ids::CONSTANT.params(1).into(),
		data: transcript.encode(),
	};

	let intent = Object {
		policies: vec![crate::ids::COIN.params(1u32)],
		unlock: crate::ids::BLS_TRANSCRIPT.params(expectation).into(),
		data: 50000u64.encode(),
	};

	let transition = Transition::<Expanded> {
		inputs: vec![intent],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000u64.encode(),
		}],
		ephemerals: vec![solution],
	};

	let machine = machine();
	let env = StaticEnvironment::default();

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(transition_exec.evaluate(&transition, &env), Ok(()));
}

#[test]
fn quark_pulse_invalid_signature() {
	const NONCE: u32 = 1170276544;
	const PUBKEY: [u8; 48] = [
		151, 8, 109, 11, 177, 216, 18, 64, 245, 104, 104, 232, 76, 155, 224, 237,
		93, 98, 143, 142, 193, 66, 98, 116, 58, 188, 94, 226, 115, 24, 19, 243,
		163, 59, 146, 86, 158, 137, 102, 126, 170, 23, 194, 132, 121, 135, 31, 33,
	];

	const MESSAGE: [u8; 36] = [
		12, 135, 143, 103, 40, 88, 12, 0, 27, 34, 0, 0, 5, 0, 0, 0, 192, 0, 193,
		69, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
	];

	const SIGNATURE: [u8; 96] = [
		181, 117, 189, 206, 29, 84, 10, 226, 101, 70, 226, 188, 18, 40, 177, 107,
		196, 67, 39, 254, 246, 162, 219, 208, 240, 65, 163, 66, 101, 70, 110, 140,
		191, 189, 244, 35, 245, 143, 118, 59, 130, 58, 1, 59, 219, 125, 63, 76, 4,
		159, 44, 194, 109, 54, 81, 40, 83, 11, 225, 130, 84, 75, 97, 195, 235, 95,
		24, 8, 52, 144, 197, 211, 145, 223, 194, 73, 174, 212, 114, 145, 211, 72,
		179, 82, 69, 225, 208, 65, 180, 175, 61, 40, 43, 104, 121, 168,
	];

	let transcript = Transcript {
		script: vec![MESSAGE.to_vec()],
		signature: Signature::decode(&mut SIGNATURE.as_slice()).unwrap(),
	};

	let expectation = ExpectedTranscript {
		signers: vec![PublicKey::decode(&mut PUBKEY.as_slice()).unwrap()],
		script: vec![ScriptEntry {
			signers: vec![0],
			expectations: vec![
				// session id
				DataPattern::<Cold>::new((4..8).equals(809000)),
				// role id
				DataPattern::<Cold>::new((8..12).greater_than(8731)),
				// uptime
				DataPattern::<Cold>::new((12..16).greater_than_or_equals(3)),
				DataPattern::<Cold>::new((12..16).less_than_or_equals(3)),
				// nonce
				DataPattern::<Cold>::new((16..20).equals(NONCE)),
				// comulative tx
				DataPattern::<Cold>::new((20..28).equals(3)),
				// cumulative rx
				DataPattern::<Cold>::new((28..36).equals(2)),
			],
		}],
	};

	let solution = Object {
		policies: vec![crate::ids::BLS_TRANSCRIPT.params(expectation.digest())],
		unlock: crate::ids::CONSTANT.params(1).into(),
		data: transcript.encode(),
	};

	let intent = Object {
		policies: vec![crate::ids::COIN.params(1u32)],
		unlock: crate::ids::BLS_TRANSCRIPT.params(expectation).into(),
		data: 50000u64.encode(),
	};

	let transition = Transition::<Expanded> {
		inputs: vec![intent],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000u64.encode(),
		}],
		ephemerals: vec![solution],
	};

	let machine = machine();
	let env = StaticEnvironment::default();

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(
		transition_exec.evaluate(&transition, &env),
		Err(EvalError::UnlockNotSatisfied(
			&transition.inputs[0],
			Location::Input
		))
	);
}

#[test]
fn quark_pulse_invalid_pattern() {
	const NONCE: u32 = 1170276544;
	const PUBKEY: [u8; 48] = [
		151, 8, 109, 11, 177, 216, 18, 64, 245, 104, 104, 232, 76, 155, 224, 237,
		93, 98, 143, 142, 193, 66, 98, 116, 58, 188, 94, 226, 115, 24, 19, 243,
		163, 59, 146, 86, 158, 137, 102, 126, 170, 23, 194, 132, 121, 135, 31, 33,
	];

	const MESSAGE: [u8; 36] = [
		12, 135, 143, 103, 40, 88, 12, 0, 27, 34, 0, 0, 5, 0, 0, 0, 192, 0, 193,
		69, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
	];

	const SIGNATURE: [u8; 96] = [
		131, 248, 115, 230, 123, 190, 234, 202, 160, 57, 41, 220, 110, 141, 17,
		193, 254, 145, 161, 74, 236, 170, 213, 151, 130, 123, 60, 111, 195, 170,
		228, 210, 59, 244, 17, 152, 196, 51, 101, 86, 2, 104, 72, 85, 112, 245,
		132, 95, 6, 8, 79, 182, 4, 44, 15, 224, 109, 131, 100, 84, 34, 31, 147,
		225, 75, 70, 77, 23, 148, 45, 253, 16, 66, 164, 159, 134, 27, 49, 53, 19,
		94, 78, 169, 147, 31, 181, 11, 197, 19, 242, 211, 55, 64, 224, 76, 136,
	];

	let transcript = Transcript {
		script: vec![MESSAGE.to_vec()],
		signature: Signature::decode(&mut SIGNATURE.as_slice()).unwrap(),
	};

	let expectation = ExpectedTranscript {
		signers: vec![PublicKey::decode(&mut PUBKEY.as_slice()).unwrap()],
		script: vec![ScriptEntry {
			signers: vec![0],
			expectations: vec![
				// session id
				DataPattern::<Cold>::new((4..8).equals(109000)),
				// role id
				DataPattern::<Cold>::new((8..12).greater_than(8731)),
				// uptime
				DataPattern::<Cold>::new((12..16).greater_than_or_equals(3)),
				DataPattern::<Cold>::new((12..16).less_than_or_equals(3)),
				// nonce
				DataPattern::<Cold>::new((16..20).equals(NONCE)),
				// comulative tx
				DataPattern::<Cold>::new((20..28).equals(3)),
				// cumulative rx
				DataPattern::<Cold>::new((28..36).equals(2)),
			],
		}],
	};

	let solution = Object {
		policies: vec![crate::ids::BLS_TRANSCRIPT.params(expectation.digest())],
		unlock: crate::ids::CONSTANT.params(1).into(),
		data: transcript.encode(),
	};

	let intent = Object {
		policies: vec![crate::ids::COIN.params(1u32)],
		unlock: crate::ids::BLS_TRANSCRIPT.params(expectation).into(),
		data: 50000u64.encode(),
	};

	let transition = Transition::<Expanded> {
		inputs: vec![intent],
		outputs: vec![Object {
			policies: vec![crate::ids::COIN.params(1)],
			unlock: crate::ids::CONSTANT.params(1).into(),
			data: 50000u64.encode(),
		}],
		ephemerals: vec![solution],
	};

	let machine = machine();
	let env = StaticEnvironment::default();

	let transition_exec = transition.instantiate(machine.factory_fn()).unwrap();
	assert_eq!(
		transition_exec.evaluate(&transition, &env),
		Err(EvalError::UnlockNotSatisfied(
			&transition.inputs[0],
			Location::Input
		))
	);
}
