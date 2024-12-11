use {
	crate::{ensure, utils::is_ephemeral},
	opto_core::*,
	opto_onchain_macros::predicate,
};

/// This predicate is used to unlock objects with a hash preimage.
///
/// It can appear in two forms:
/// - As an unlock expression on an object, in which case it is used guard an
///   object and allow consumption of the object only if the state transition
///   has a transient object with `preimage` policy with the hash paramer equal
///   to the unlock hash and contents with the preimage.
/// - As a policy on a transient object, in which case it is used as a
///   fullfillment of an unlock condition of an object.
#[predicate(id = 202, core_crate = opto_core)]
pub fn blake2b_256(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	if params.len() != Digest::SIZE {
		return false;
	}

	match ctx.role {
		Role::Policy(_, _) => {
			ensure!(is_ephemeral(&ctx));
			Digest::compute(&ctx.object.data).as_ref() == params
		}
		Role::Unlock(me, _) => {
			// for this to evaluate to true we need to have a transinet
			// object with the preimage policy and the hash parameter equal
			// to the unlock hash and the contents equal to the preimage
			transition.ephemerals.iter().any(|obj| {
				obj
					.policies
					.iter()
					.any(|policy| policy.id == me.id && policy.params == params)
			})
		}
	}
}

#[cfg(test)]
mod test {
	use {super::*, crate::native_impl_factory, opto_core::test::*};

	#[test]
	fn smoke() {
		let data = b"hello world";
		let hashed = Digest::compute(data);
		let locked_object = ObjectBuilder::new()
			.with_unlock(
				Predicate {
					id: PredicateId(202),
					params: hashed.to_vec(),
				}
				.into(),
			)
			.build();

		let mut transition = Transition {
			inputs: vec![locked_object],
			ephemerals: vec![],
			outputs: vec![],
		};

		let env = StaticEnvironment::default();
		let instance = transition.instantiate(native_impl_factory).unwrap();

		// this should fail because there is no transient
		// input object that unlocks this predicate.
		let evaluation = instance.evaluate(&transition, &env);
		assert_eq!(
			evaluation,
			Err(EvalError::UnlockNotSatisfied(
				&transition.inputs[0],
				Location::Input
			))
		);

		let unlocking_object = ObjectBuilder::new()
			.with_policy(Predicate {
				id: PredicateId(202),
				params: hashed.to_vec(),
			})
			.with_data(data.to_vec())
			.build();

		transition.ephemerals.push(unlocking_object);

		// this should pass because we have a trnasient object that unlocks
		// the input object preimage predicate.
		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition, &env);
		assert_eq!(evaluation, Ok(()));

		// replace it with a different preimage that is invalid
		let invalid_unlocking_object = ObjectBuilder::new()
			.with_policy(Predicate {
				id: PredicateId(202),
				params: hashed.to_vec(),
			})
			.with_data(data.iter().copied().rev().collect::<Vec<_>>())
			.build();

		transition.ephemerals[0] = invalid_unlocking_object;

		// this should fail because the preimage value does not hash to the
		// param value.
		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition, &env);
		assert_eq!(
			evaluation,
			Err(EvalError::PolicyNotSatisfied(
				&transition.ephemerals[0],
				Location::Ephemeral,
				0
			))
		);
	}
}
