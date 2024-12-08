use {core::time::Duration, opto_core::*, opto_onchain::*, scale::Decode};

/// Predicate that checks if the current time is on or after a given timestamp.
///
/// param: u32 - timestamp in seconds since epoch
#[predicate(id = 402, core_crate = opto_core)]
pub fn after_time(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let Ok(timestamp) = u32::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.time_now() >= Duration::from_secs(timestamp as u64)
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

		let mut env = StaticEnvironment::default();
		env.block_number = 20;
		env.times.insert(20, Duration::from_secs(4000)); // before unlock
		env.times.insert(25, Duration::from_secs(6000)); // after unlock

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

		let mut env = StaticEnvironment::default();
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
