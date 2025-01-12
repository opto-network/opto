#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeSet, vec::Vec};
#[cfg(feature = "std")]
use std::collections::BTreeSet;

use {super::*, ipld_nostd::CarReader};

pub fn install<T: Config<I>, I: 'static>(
	origin: OriginFor<T>,
	bytecode_or_car: Vec<u8>,
) -> DispatchResult {
	let origin = ensure_signed_or_root(origin.clone())?;
	let is_invoked_by_root = origin.is_none();

	// we are installing one predicate
	// this is the standard WASM magic number prefix per the WASM spec
	if bytecode_or_car.starts_with(&[0x00, 0x61, 0x73, 0x6d]) {
		install_one_predicate::<T, I>(bytecode_or_car, is_invoked_by_root)?;
	} else {
		// check if the archive is within the allowed size limit. applies only
		// to non-root calls. Genesis archive is allowed to be larger.
		if !is_invoked_by_root {
			ensure!(
				bytecode_or_car.len() <= T::MaximumArchiveSize::get() as usize,
				Error::<T, I>::PredicateArchiveTooLarge
			);
		}

		// seems like we are installing a group of predicates in a CAR file
		let reader = CarReader::new(core2::io::Cursor::new(bytecode_or_car))
			.map_err(|_| Error::<T, I>::InvalidPredicateArchive)?;
		let roots: BTreeSet<_> = reader.header().roots().iter().cloned().collect();

		// for each predicate in the CAR file, install it
		// if any of the predicates fail to install, the whole operation fails
		// and nothing gets installed.
		for predicate in reader {
			let Ok((cid, wasm)) = predicate else {
				return Err(Error::<T, I>::InvalidPredicateArchive.into());
			};

			if roots.contains(&cid) {
				continue;
			}

			install_one_predicate::<T, I>(wasm, is_invoked_by_root)?;
		}
	}

	Ok(())
}

fn install_one_predicate<T: Config<I>, I: 'static>(
	bytecode: Vec<u8>,
	is_invoked_by_root: bool,
) -> DispatchResult {
	// check if the bytecode is within the allowed size limit. applies only to
	// non-root calls
	if !is_invoked_by_root {
		ensure!(
			bytecode.len() <= T::MaximumPredicateSize::get() as usize,
			Error::<T, I>::PredicateTooLarge
		);
	}

	// ensure that the bytecode is valid wasm and adheres to public ABI.
	let predicate_id = vm::validate(&bytecode)
		.map_err(|e| Error::<T, I>::InvalidPredicateCode(e))?;

	// ensure that the predicate id is not reserved for system predicates,
	// unless the call is made by root (genesis & stdpred installer is called by
	// root).
	if predicate_id <= T::ReservedPredicateIds::get() && !is_invoked_by_root {
		return Err(Error::<T, I>::InvalidPredicateId.into());
	}

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
