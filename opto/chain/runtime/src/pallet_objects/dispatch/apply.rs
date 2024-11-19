use {
	super::*,
	frame::prelude::*,
	repr::{Compact, Expanded},
	vm::PredicateMachine,
};

pub fn apply<T: Config<I>, I: 'static>(
	origin: OriginFor<T>,
	transitions: vec::Vec<Transition<Compact>>,
) -> DispatchResult {
	let _ = ensure_signed(origin)?;

	let mut consumed = collections::BTreeMap::new();
	let mut produced = collections::BTreeMap::new();

	for transition in transitions {
		// keep track of all consumed and produced objects
		// we want to emit events for them all at the end, except transient objects
		// that were produced and consumed in the same transition.
		for input in &transition.inputs {
			*consumed.entry(*input).or_insert(0) += 1;
			*produced.entry(*input).or_insert(0) -= 1;
		}

		for output in &transition.outputs {
			let output = output.digest();
			*produced.entry(output).or_insert(0) += 1;
			*consumed.entry(output).or_insert(0) -= 1;
		}

		// at-rest version of the transition. This is where all the data, parameters
		// and other non-executable pieces of the transition are stored.
		let expanded = expand::<T, I>(transition)?;

		// an executable version of the predicate that has runnable predicate code
		// and references to the at-rest version of the transition.
		let instantiated = expanded.instantiate(|pred: &AtRest| {
			// load predicate wasm code
			let bytecode = Predicates::<T, I>::get(pred.id)
				.ok_or(Error::<T, I>::PredicateNotFound)?;

			// create an executable version that can be evaluated
			// Safety: Predicate was validated when it was installed.
			let instance = unsafe {
				PredicateMachine::new_unchecked(bytecode.as_slice(), pred.id)
					.expect("predicate was validated when installed")
			};
			Ok::<_, Error<T, I>>(instance.functor())
		})?;

		// Evaluate the transition. If this does not return any error, the
		// transition is valid, and we can proceed with state changes.
		instantiated.evaluate(&expanded).map_err(|e| match e {
			opto_core::eval::Error::PolicyNotSatisfied(_, _, _) => {
				Error::<T, I>::UnsatifiedPolicy
			}
			opto_core::eval::Error::UnlockNotSatisfied(_, _) => {
				Error::<T, I>::UnsatifiedUnlockExpression
			}
			_ => unreachable!(),
		})?;

		// all good, now persist all output objects
		for output in expanded.outputs {
			produce_output::<T, I>(output, false)?;
		}
	}

	// now emit events for all consumed and produced objects
	// skip objects that were produced and consumed in the same transition
	for (digest, balance) in consumed {
		if balance <= 0 {
			continue;
		}

		for _ in 0..balance {
			Pallet::<T, I>::deposit_event(Event::ObjectDestroyed { digest });
		}
	}

	for (digest, balance) in produced {
		if balance <= 0 {
			continue;
		}

		let object = Objects::<T, I>::get(digest) // must exist
			.expect("just produced object must exist");

		for _ in 0..balance {
			Pallet::<T, I>::deposit_event(Event::ObjectCreated {
				object: object.object.clone(),
			});
		}
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
			.map(|digest| consume_input::<T, I>(digest, false))
			.collect::<Result<_, _>>()?,
		ephemerals: transition.ephemerals,
		outputs: transition.outputs,
	})
}
