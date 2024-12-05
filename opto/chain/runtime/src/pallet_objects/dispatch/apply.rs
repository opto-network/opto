use {
	super::*,
	frame::prelude::*,
	vm::{instantiate, OnChainEnvironment},
};

pub fn apply<T: Config<I>, I: 'static>(
	origin: OriginFor<T>,
	transitions: vec::Vec<Transition<Compact>>,
) -> DispatchResult {
	let _ = ensure_signed(origin)?;

	let env = OnChainEnvironment::<T, I>::new();

	for transition in transitions {
		// at-rest expanded version of the transition. This is where all the data,
		// parameters and other non-executable pieces of the transition are
		// stored. This will also consume all input objects from state.
		let expanded = expand::<T, I>(transition.clone())?;

		// an executable version of the predicate that has runnable predicate code
		// and references to the at-rest version of the transition.
		let instantiated = expanded.instantiate(|pred: &AtRest| {
			// load predicate wasm code
			let bytecode = Predicates::<T, I>::get(pred.id)
				.ok_or(Error::<T, I>::PredicateNotFound)?;

			// create an executable version that can be evaluated
			// Safety: Predicate was validated when it was installed.
			Ok::<_, Error<T, I>>(unsafe { instantiate(bytecode.as_slice(), pred.id) })
		})?;

		// Evaluate the transition. If this does not return any error, the
		// transition is valid, and we can proceed with state changes.
		instantiated
			.evaluate(&expanded, &env)
			.map_err(|e| match e {
				opto_core::eval::Error::PolicyNotSatisfied(_, _, pos) => {
					Error::<T, I>::UnsatifiedPolicy(pos as u8)
				}
				opto_core::eval::Error::UnlockNotSatisfied(_, _) => {
					Error::<T, I>::UnsatifiedUnlockExpression
				}
				_ => unreachable!(),
			})?;

		// all good, now persist all output objects
		for output in expanded.outputs {
			produce_output::<T, I>(output)?;
		}

		Pallet::<T, I>::deposit_event(Event::StateTransitioned { transition });
	}

	Ok(())
}

fn expand<T: Config<I>, I: 'static>(
	transition: Transition<Compact>,
) -> Result<Transition<Expanded>, Error<T, I>> {
	Ok(Transition::<Expanded> {
		inputs: transition
			.inputs
			.into_iter()
			.map(|digest| consume_input::<T, I>(digest))
			.collect::<Result<_, _>>()?,
		ephemerals: transition.ephemerals,
		outputs: transition.outputs,
	})
}
