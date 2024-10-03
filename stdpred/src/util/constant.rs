use opto::{
	eval::{Context, Location, Role},
	transition::Transition,
};

/// Constant
///
/// When used as in an unlock expression, it will evaluate to true if the first
/// byte of the params is 1
///
/// When used as a policy of an input object, it will evaluate to true only if
/// the same object is also present in the outputs of the transition. This gives
/// us a way to have immutable and immortal objects.
///
/// When used on an ephemeral or output object policy, it will always evaluate
/// to true.
#[opto::predicate(id = 100)]
pub fn constant(
	ctx: Context<'_>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	match ctx.role {
		Role::Policy(_, _) => match ctx.location {
			Location::Input => transition.outputs.contains(ctx.object),
			Location::Output => true,
			Location::Ephemeral => true,
		},
		Role::Unlock(_, _) => params.first().map(|&x| x == 1).unwrap_or(false),
	}
}

#[cfg(test)]
mod tests {
	use {
		crate::native_impl_factory,
		alloc::vec,
		opto::{eval::Location, test::*, transition::Error, Transition},
	};

	#[test]
	fn unlock_smoke_negative() {
		let input = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[0]).into(),
			1000u64.to_le_bytes().to_vec(),
		);
		let output = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[0]).into(),
			1000u64.to_le_bytes().to_vec(),
		);

		let transition = Transition {
			inputs: vec![input],
			ephemerals: vec![],
			outputs: vec![output],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(
			evaluation,
			Err(Error::UnlockNotSatisfied(
				&transition.inputs[0],
				Location::Input
			))
		);
	}

	#[test]
	fn unlock_smoke_positive() {
		let input = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
			1000u64.to_le_bytes().to_vec(),
		);
		let output = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
			1000u64.to_le_bytes().to_vec(),
		);

		let transition = Transition {
			inputs: vec![input],
			ephemerals: vec![],
			outputs: vec![output],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}

	#[test]
	fn policy_smoke_positive() {
		let input1 = opto::test::object(
			vec![predicate(100, b"const-obj-1")],
			predicate(100, &[1]).into(),
			b"const-obj-data-1".to_vec(),
		);

		let input2 = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
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

		let output3 = opto::test::object(
			vec![predicate(100, b"const-obj-1")],
			predicate(100, &[1]).into(),
			b"const-obj-data-1".to_vec(),
		);

		let transition = Transition {
			inputs: vec![input1, input2],
			ephemerals: vec![],
			outputs: vec![output1, output2, output3],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(evaluation, Ok(()));
	}

	#[test]
	fn policy_smoke_negative() {
		let input1 = opto::test::object(
			vec![predicate(100, b"const-obj-1")],
			predicate(100, &[1]).into(),
			b"const-obj-data-1".to_vec(),
		);

		let input2 = opto::test::object(
			vec![predicate(1000, b"USDT")],
			predicate(100, &[1]).into(),
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

		let transition = Transition {
			inputs: vec![input1, input2],
			ephemerals: vec![],
			outputs: vec![output1, output2],
		};

		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);

		assert_eq!(
			evaluation,
			Err(Error::PolicyNotSatisfied(
				&transition.inputs[0],
				Location::Input,
				0
			))
		);
	}
}
