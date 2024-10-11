use {
	crate::{ensure, utils::is_ephemeral},
	opto::{Context, Digest, Role, Transition},
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
#[opto::predicate(id = 202)]
pub fn blake2b_256(
	ctx: Context<'_>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	if params.len() != opto::Digest::SIZE {
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
	use {
		super::*,
		crate::native_impl_factory,
		opto::{
			repr::AtRest,
			test::ObjectBuilder,
			transition::Error,
			Digest,
			Location,
			PredicateId,
		},
	};

	#[test]
	fn smoke() {
		let data = b"hello world";
		let hashed = Digest::compute(data);
		let locked_object = ObjectBuilder::new()
			.with_unlock(
				AtRest {
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

		let instance = transition.instantiate(native_impl_factory).unwrap();

		// this should fail because there is no transient
		// input object that unlocks this predicate.
		let evaluation = instance.evaluate(&transition);
		assert_eq!(
			evaluation,
			Err(Error::UnlockNotSatisfied(
				&transition.inputs[0],
				Location::Input
			))
		);

		let unlocking_object = ObjectBuilder::new()
			.with_policy(AtRest {
				id: PredicateId(202),
				params: hashed.to_vec(),
			})
			.with_data(data.to_vec())
			.build();

		transition.ephemerals.push(unlocking_object);

		// this should pass because we have a trnasient object that unlocks
		// the input object preimage predicate.
		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);
		assert_eq!(evaluation, Ok(()));

		// replace it with a different preimage that is invalid
		let invalid_unlocking_object = ObjectBuilder::new()
			.with_policy(AtRest {
				id: PredicateId(202),
				params: hashed.to_vec(),
			})
			.with_data(data.iter().copied().rev().collect::<Vec<_>>())
			.build();

		transition.ephemerals[0] = invalid_unlocking_object;

		// this should fail because the preimage value does not hash to the
		// param value.
		let instance = transition.instantiate(native_impl_factory).unwrap();
		let evaluation = instance.evaluate(&transition);
		assert_eq!(
			evaluation,
			Err(Error::PolicyNotSatisfied(
				&transition.ephemerals[0],
				Location::Ephemeral,
				0
			))
		);
	}
}
