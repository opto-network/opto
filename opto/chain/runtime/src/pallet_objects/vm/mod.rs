#[cfg(not(any(feature = "std", test)))]
extern crate alloc;

#[cfg(not(any(feature = "std", test)))]
use alloc::{boxed::Box, vec};

use {
	env::attach_syscalls,
	eval::PredicateFunctor,
	frame::{log::error, prelude::*},
	frame_support::PalletError,
	opto_core::*,
	scale::{Decode, Encode},
	scale_info::TypeInfo,
	wasmi::{core::ValType, *},
};

mod env;

pub use env::{OnChainEnvironment, ValidationEnv};

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo, PalletError)]
pub enum Error {
	/// WASM code is not valid.
	///
	/// This also includes missing required exports or not exporting memory.
	InvalidCode,

	/// Wasm code is not exporting  required ext.
	MissingExport,

	/// The exported value is not a valid global value.
	///
	/// This happens when the predicate id export is not pointing to a valid u32.
	InvalidGlobalExportValue,

	/// The exported function has invalid signature.
	InvalidFuncExportSignature,

	/// The code is importing something not exported.
	InvalidImport,

	/// The code is importing a non-exported function.
	InvalidFuncImport,

	/// The memory is not exported.
	MemoryNotExported,
}

/// Instantiates a new VirtualMachine with the given predicate bytecode.
///
/// This method will extract the declared predicate id from the bytecode and
/// ensure that all exports are present and valid. This validation happens
/// during installation of the predicate.
///
/// Once validated the predicate is installed in the pallet storage and
/// subsequent instances are created using the `new_unchecked` method.
pub fn validate(bytecode: &[u8]) -> Result<PredicateId, Error> {
	let engine = Engine::default();
	let module = Module::new(&engine, bytecode) //
		.map_err(|_| Error::InvalidCode)?;

	let mut store = Store::new(&engine, &ValidationEnv);
	let instance = attach_syscalls(&engine, &module, &mut store, PredicateId(0))
		.map_err(|_| Error::InvalidCode)?;

	let Some(Val::I32(pred_id_address)) = instance
		.get_global(&store, "_pred_id")
		.map(|g| g.get(&store))
	else {
		return Err(Error::MissingExport);
	};

	// Extract the predicate id from the exported global value.
	let Some(Ok(pred_id)) = instance.get_memory(&store, "memory").map(|memory| {
		let slice = memory.data(&store);
		let slice_len = size_of::<u32>();
		let bytes_range =
			pred_id_address as usize..pred_id_address as usize + slice_len;
		slice[bytes_range]
			.try_into()
			.map(|bytes: [u8; 4]| u32::from_le_bytes(bytes))
	}) else {
		return Err(Error::InvalidGlobalExportValue);
	};

	// check if the required abi is exported by the wasm code.
	ensure_func_abi(
		&instance,
		&store,
		"_eval",
		&[ValType::I32; 8][..],
		&[ValType::I32][..],
	)?;

	ensure_func_abi(&instance, &store, "_alloc", &[ValType::I32], &[
		ValType::I32,
	])?;

	ensure_func_abi(
		&instance,
		&store,
		"_dealloc",
		&[ValType::I32, ValType::I32],
		&[],
	)?;

	// ensure the predicate code is exporting its memory
	if instance.get_memory(&store, "memory").is_none() {
		return Err(Error::MemoryNotExported);
	}

	Ok(PredicateId(pred_id))
}

/// Given a bytecode and a predicate id of an installed and validated predicate,
/// this function will instantiate a new PredicateFunctor that is ready to be
/// invoked with a context, transition and params to evaluate a state
/// transition.
///
/// SAFETY:
/// 1. It is the caller's responsibility to ensure that the bytecode is valid
///    and all invariants that are checked during installation are met. As a
///    rule of thumb only invoke this function on installed predicates.
///
/// 2. It is the caller's responsibility that the context, transition and params
///    are well formed. As a rule of thumb, if this is invoked by
///    `core::transition::Transition::evaluate` then you should be fine.
pub unsafe fn instantiate<E: Environment>(
	bytecode: impl AsRef<[u8]>,
	id: PredicateId,
) -> PredicateFunctor<E> {
	let engine = Engine::default();
	let module = Module::new_unchecked(&engine, bytecode.as_ref())
		.expect("WASM validated during installation");

	Box::new(
		move |context: Context<E>,
		      transition: &Transition<Expanded>,
		      params: &[u8]|
		      -> bool {
			let mut store = Store::new(&engine, context.env);
			let instance = attach_syscalls(&engine, &module, &mut store, id)
				.expect("validated during installation");

			let func_eval = instance
				.get_func(&mut store, "_eval")
				.expect("_eval existance validated during installation");

			let func_alloc = instance
				.get_func(&store, "_alloc")
				.expect("_alloc existance validated during installation");

			let memory = instance
				.get_memory(&store, "memory")
				.expect("exported memory validated during installation");

			let transition_bytes = transition.encode();

			let location = match context.location {
				Location::Input => 0,
				Location::Ephemeral => 1,
				Location::Output => 2,
			};

			let object_index = match context.location {
				Location::Input => index_of(&transition.inputs, context.object),
				Location::Ephemeral => index_of(&transition.ephemerals, context.object),
				Location::Output => index_of(&transition.outputs, context.object),
			}
			.expect("object not found in transition");

			let (pred_role, pred_index) = match context.role {
				Role::Policy(_, index) => (0i32, index as i32),
				Role::Unlock(_, index) => (1i32, index as i32),
			};

			let required_bytes = transition_bytes.len() as i32
				+ params.len() as i32
				+ size_of::<u32>() as i32 // location
				+ size_of::<u32>() as i32 // predicate index
				+ size_of::<u32>() as i32; // role

			// Allocate memory for the transition and params.
			let mut alloc_result = vec![Val::I32(0)];
			if func_alloc
				.call(&mut store, &[Val::I32(required_bytes)], &mut alloc_result)
				.is_err()
			{
				error!(
					"Failed to allocate memory for transition {transition:?} and params \
					 {params:?} in predicate {id}",
				);
				return false;
			};

			let memptr = match alloc_result[0] {
				Val::I32(ptr) => ptr as usize,
				_ => unreachable!(),
			};

			// Write the transition bytes to the allocated memory.
			if memory
				.write(&mut store, memptr, transition_bytes.as_slice())
				.is_err()
			{
				error!(
					"Failed to write transition {transition:?} bytes to memory \
					 {memptr:?} in predicate {id}",
				);
				return false;
			}

			// Write the params to the allocated memory.
			if memory
				.write(&mut store, memptr + transition_bytes.len(), params)
				.is_err()
			{
				error!(
					"Failed to write params {params:?} to memory {memptr:?} in \
					 predicate {id}",
				);
				return false;
			}

			let mut result = vec![Val::I32(0)];
			let Ok(_) = func_eval.call(
				&mut store,
				&[
					Val::I32(location),
					Val::I32(object_index as i32),
					Val::I32(pred_role),
					Val::I32(pred_index),
					Val::I32(memptr as i32),
					Val::I32(transition_bytes.len() as i32),
					Val::I32(memptr as i32 + transition_bytes.len() as i32),
					Val::I32(params.len() as i32),
				],
				&mut result,
			) else {
				error!("Failed to call eval function");
				return false;
			};

			matches!(result[0], Val::I32(1))
		},
	)
}

fn ensure_func_abi(
	instance: &Instance,
	store: &Store<&ValidationEnv>,
	func_name: &str,
	expected_params: &[ValType],
	expected_results: &[ValType],
) -> Result<(), Error> {
	let func = instance
		.get_func(store, func_name)
		.map(|f| f.ty(store))
		.ok_or(Error::MissingExport)?;

	if func.params() != expected_params || func.results() != expected_results {
		return Err(Error::InvalidFuncExportSignature);
	}

	Ok(())
}

fn index_of_unchecked<T>(slice: &[T], item: &T) -> usize {
	if size_of::<T>() == 0 {
		return 0; // do what you will with this case
	}
	(item as *const T as *const () as usize
		- slice.as_ptr() as *const () as usize)
		/ size_of::<T>()
}

fn index_of<T>(slice: &[T], item: &T) -> Option<usize> {
	let ptr = item as *const T;
	let len = slice.len() as isize;
	// SAFETY: `ptr` is a valid pointer to `T` and `len` is the length of `slice`.
	// We are not dereferencing `ptr` and we are not accessing any element of
	// `slice` beyond the end of the slice. We're only doing pointer arithmetic.
	let slice_end = unsafe { slice.as_ptr().offset(len) };
	if slice.as_ptr() <= ptr && slice_end > ptr {
		Some(index_of_unchecked(slice, item))
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use {super::*, eval::InUse, opto_core::env::StaticEnvironment};

	#[test]
	fn invalid_bytecode_returns_error() {
		let invalid_bytecode = [0x00, 0x01, 0x02];
		assert!(matches!(
			super::validate(&invalid_bytecode),
			Err(super::Error::InvalidCode)
		));
	}

	#[test]
	fn extract_predicate_id() {
		let bytecode = include_bytes!("../tests/assets/101.wasm");
		let predicate_id = super::validate(bytecode.as_slice()).unwrap();
		assert_eq!(predicate_id, PredicateId(101));

		let bytecode = include_bytes!("../tests/assets/100.wasm");
		let predicate_id = super::validate(bytecode.as_slice()).unwrap();
		assert_eq!(predicate_id, PredicateId(100));

		let bytecode = include_bytes!("../tests/assets/201.wasm");
		let predicate_id = super::validate(bytecode.as_slice()).unwrap();
		assert_eq!(predicate_id, PredicateId(201));
	}

	#[test]
	fn eval_const_predicate_false_in_policy() {
		let bytecode = include_bytes!("../tests/assets/100.wasm");
		let func1 = unsafe { instantiate(bytecode.as_slice(), PredicateId(0)) };
		let func2 = unsafe { instantiate(bytecode.as_slice(), PredicateId(0)) };

		let object1 = Object {
			policies: vec![Predicate {
				id: PredicateId(0),
				params: vec![0],
			}],
			unlock: vec![Op::Predicate(Predicate {
				id: PredicateId(0),
				params: vec![0],
			})]
			.try_into()
			.unwrap(),
			data: vec![0],
		};

		let transition = Transition {
			inputs: vec![object1.clone()],
			ephemerals: vec![],
			outputs: vec![],
		};

		let object1_inst = Object {
			policies: vec![InUse::new(func1)],
			unlock: InUse::new(func2).into(),
			data: object1.data.as_slice(),
		};

		let transition_inst = Transition {
			inputs: vec![object1_inst],
			ephemerals: vec![],
			outputs: vec![],
		};

		let env = StaticEnvironment::default();
		let evaluation = transition_inst.evaluate(&transition, &env);

		assert_eq!(
			evaluation,
			Err(opto_core::eval::Error::PolicyNotSatisfied(
				&transition.inputs[0],
				Location::Input,
				0
			))
		);
	}

	#[test]
	fn eval_const_predicate_false_in_unlock() {
		let bytecode100 = include_bytes!("../tests/assets/100.wasm");
		let bytecode502 = include_bytes!("../tests/assets/502.wasm");
		let env = StaticEnvironment::default();

		let func1 =
			unsafe { instantiate(bytecode100.as_slice(), PredicateId(100)) };

		let func2 =
			unsafe { instantiate(bytecode502.as_slice(), PredicateId(502)) };

		let object1 = Object {
			policies: vec![Predicate {
				id: PredicateId(502),
				params: vec![1],
			}],
			unlock: vec![Op::Predicate(Predicate {
				id: PredicateId(100),
				params: vec![0],
			})]
			.try_into()
			.unwrap(),
			data: vec![0],
		};

		let transition = Transition {
			inputs: vec![object1.clone()],
			ephemerals: vec![],
			outputs: vec![],
		};

		let object1_inst = Object {
			policies: vec![InUse::new(func2)],
			unlock: InUse::new(func1).into(),
			data: object1.data.as_slice(),
		};

		let transition_inst = Transition {
			inputs: vec![object1_inst],
			ephemerals: vec![],
			outputs: vec![],
		};

		let evaluation = transition_inst.evaluate(
			&transition, //
			&env,
		);

		assert_eq!(
			evaluation,
			Err(opto_core::eval::Error::UnlockNotSatisfied(
				&transition.inputs[0],
				Location::Input,
			))
		);
	}

	#[test]
	fn eval_const_predicate_true() {
		let env = StaticEnvironment::default();
		let bytecode100 = include_bytes!("../tests/assets/100.wasm");
		let bytecode502 = include_bytes!("../tests/assets/502.wasm");

		let func1 =
			unsafe { instantiate(bytecode100.as_slice(), PredicateId(100)) };

		let func2 =
			unsafe { instantiate(bytecode502.as_slice(), PredicateId(502)) };

		let object1 = Object {
			policies: vec![Predicate {
				id: PredicateId(502),
				params: vec![1],
			}],
			unlock: vec![Op::Predicate(Predicate {
				id: PredicateId(100),
				params: vec![1],
			})]
			.try_into()
			.unwrap(),
			data: vec![0],
		};

		let transition = Transition {
			inputs: vec![object1.clone()],
			ephemerals: vec![],
			outputs: vec![],
		};

		let object1_inst = Object {
			policies: vec![InUse::new(func2)],
			unlock: InUse::new(func1).into(),
			data: object1.data.as_slice(),
		};

		let transition_inst = Transition {
			inputs: vec![object1_inst],
			ephemerals: vec![],
			outputs: vec![],
		};

		let evaluation = transition_inst.evaluate(
			&transition, //
			&env,
		);

		assert_eq!(evaluation, Ok(()));
	}
}
