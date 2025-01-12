use {
	core::time::Duration,
	opto_core::*,
	opto_onchain_macros::*,
	scale::Decode,
};

/// Predicate that checks if the current time is on or after a given timestamp.
///
/// param: u32 - timestamp in milliseconds since epoch
#[predicate(id = 402, core_crate = opto_core)]
pub fn after_time(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let Ok(timestamp) = Duration::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.time_now() >= timestamp
}

#[predicate(id = 403, core_crate = opto_core)]
pub fn after_block(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let Ok(block) = u32::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.block_number() >= block
}

#[cfg(test)]
mod test {
	use {
		super::*,
		crate::native_impl_factory,
		opto_core::test::*,
		scale::Encode,
	};

	#[test]
	fn after_time() {
		let locked_object = ObjectBuilder::new()
			.with_unlock(
				Predicate {
					id: PredicateId(402),
					params: 6000.encode(),
				}
				.into(),
			)
			.build();

		let transition = Transition {
			inputs: vec![locked_object],
			ephemerals: vec![],
			outputs: vec![],
		};

		let mut env = StaticEnvironment {
			block_number: 20,
			times: [
				(20, Duration::from_secs(4000)), // before unlock
				(25, Duration::from_secs(6000)), // after unlock
			]
			.into_iter()
			.collect(),
			..Default::default()
		};

		{
			// this should fail because the time is before the unlock time
			let evaluation = transition
				.instantiate(native_impl_factory)
				.unwrap()
				.evaluate(&transition, &env);

			assert_eq!(
				evaluation,
				Err(EvalError::UnlockNotSatisfied(
					&transition.inputs[0],
					Location::Input
				))
			);
		}

		// move to a block with timestamp after the unlock
		env.block_number = 25;

		{
			// this should pass because the time is after the unlock time
			let evaluation = transition
				.instantiate(native_impl_factory)
				.unwrap()
				.evaluate(&transition, &env);

			assert_eq!(evaluation, Ok(()));
		}
	}

	#[test]
	fn after_block() {
		let locked_object = ObjectBuilder::new()
			.with_unlock(
				Predicate {
					id: PredicateId(403),
					params: 60.encode(),
				}
				.into(),
			)
			.build();

		let transition = Transition {
			inputs: vec![locked_object],
			ephemerals: vec![],
			outputs: vec![],
		};

		let mut env = StaticEnvironment {
			block_number: 20,
			..Default::default()
		};

		{
			// this should fail because the block no is lower than the unlock block
			let evaluation = transition
				.instantiate(native_impl_factory)
				.unwrap()
				.evaluate(&transition, &env);

			assert_eq!(
				evaluation,
				Err(EvalError::UnlockNotSatisfied(
					&transition.inputs[0],
					Location::Input
				))
			);
		}
		env.block_number = 20;

		{
			// this should fail because the time is before the unlock time
			let evaluation = transition
				.instantiate(native_impl_factory)
				.unwrap()
				.evaluate(&transition, &env);

			assert_eq!(
				evaluation,
				Err(EvalError::UnlockNotSatisfied(
					&transition.inputs[0],
					Location::Input
				))
			);
		}

		// move to a block to a number after the unlock
		env.block_number = 62;

		{
			// this should pass because the block no is higher than the unlock block
			let evaluation = transition
				.instantiate(native_impl_factory)
				.unwrap()
				.evaluate(&transition, &env);

			assert_eq!(evaluation, Ok(()));
		}
	}
}
