#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use {super::*, frame::prelude::*, vm::PredicateMachine};

pub fn install<T: Config<I>, I: 'static>(
	origin: OriginFor<T>,
	bytecode: Vec<u8>,
) -> DispatchResult {
	let _ = ensure_signed_or_root(origin)?;

	// check if the bytecode is within the allowed size limit
	ensure!(
		bytecode.len() <= T::MaximumPredicateSize::get() as usize,
		Error::<T, I>::PredicateTooLarge
	);

	// ensure that the bytecode is valid wasm and adheres to public ABI.
	let predicate_id = PredicateMachine::validate(&bytecode)
		.map_err(|e| Error::<T, I>::InvalidPredicateCode(e))?;

	// ensure that the predicate id is not already taken
	if Predicates::<T, I>::contains_key(predicate_id) {
		return Err(Error::<T, I>::PredicateAlreadyExists.into());
	}

	// store the predicate
	Predicates::<T, I>::insert(predicate_id, bytecode);

	// Emit an event that a new predicate was installed
	Pallet::<T, I>::deposit_event(Event::PredicateInstalled { id: predicate_id });

	Ok(())
}
