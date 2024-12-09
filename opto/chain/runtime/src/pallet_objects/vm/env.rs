#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(any(test, feature = "std"))]
use wasmi::AsContext;
use {
	crate::pallet_objects::{Config, Timestamp, Vrf},
	core::{marker::PhantomData, time::Duration},
	frame::prelude::*,
	opto_core::{env::Environment, Digest, PredicateId},
	sp_core::U256,
	wasmi::{
		AsContextMut,
		Caller,
		Engine,
		Func,
		Instance,
		Linker,
		Module,
		Store,
	},
};

/// Implements VM syscalls that are available to on-chain predicates
/// during evaluation.
pub struct OnChainEnvironment<T, I>(PhantomData<(T, I)>);

impl<T: Config<I>, I: 'static> Clone for OnChainEnvironment<T, I> {
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<T: Config<I>, I: 'static> OnChainEnvironment<T, I> {
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config<I>, I: 'static> Environment for OnChainEnvironment<T, I> {
	fn block_number(&self) -> u32 {
		let block_no: U256 = frame_system::Pallet::<T>::block_number().into();
		block_no.try_into() // fix it in about 800 years.
    .expect("block number too large")
	}

	fn vrf_at(&self, block: u32) -> Option<Digest> {
		Vrf::<T, I>::get(block)
	}

	fn time_at(&self, block: u32) -> Option<Duration> {
		let moment = Timestamp::<T, I>::get(block)?;
		Some(Duration::from_millis(moment))
	}

	fn history_len(&self) -> u32 {
		T::HistoryLength::get()
	}
}

/// Implements VM syscalls that are available to on-chain predicates
/// during validation / installation of a predicate.
///
/// During installation no functions in the predicate are executed, only
/// exported globals are read. This implementation does not provide any
/// runtime logic.
pub struct ValidationEnv;

impl Environment for ValidationEnv {
	fn block_number(&self) -> u32 {
		unimplemented!()
	}

	fn history_len(&self) -> u32 {
		unimplemented!()
	}

	fn time_at(&self, _: u32) -> Option<core::time::Duration> {
		unimplemented!()
	}

	fn vrf_at(&self, _: u32) -> Option<opto_core::Digest> {
		unimplemented!()
	}
}

pub fn attach_syscalls<'e, E: Environment + 'e>(
	engine: &Engine,
	module: &Module,
	store: &mut Store<&'e E>,
	id: PredicateId,
) -> Result<Instance, wasmi::Error> {
	let vrf_at = Func::wrap(
		store.as_context_mut(),
		|caller: Caller<'_, &'e E>, block: u32| {
			return to_tuple(&caller.data().vrf_at(block).unwrap_or(Digest::ZERO));
		},
	);

	let time_at = Func::wrap(
		store.as_context_mut(),
		|caller: Caller<'_, &'e E>, block: u32| {
			return caller
				.data()
				.time_at(block)
				.unwrap_or(Duration::ZERO)
				.as_secs() as u32;
		},
	);

	let history_len =
		Func::wrap(store.as_context_mut(), |caller: Caller<'_, &'e E>| {
			caller.data().history_len()
		});

	let block_number =
		Func::wrap(store.as_context_mut(), |caller: Caller<'_, &'e E>| {
			caller.data().block_number()
		});

	#[cfg(any(test, feature = "std"))]
	let debug = Func::wrap(
		store.as_context_mut(),
		move |caller: Caller<'_, &'e E>, ptr: u32, len: u32| {
			let Some(memory) =
				caller.get_export("memory").and_then(|e| e.into_memory())
			else {
				println!("debug: memory export not found");
				return;
			};

			let mut buffer = vec![0u8; len as usize];

			if memory
				.read(caller.as_context(), ptr as usize, &mut buffer)
				.is_err()
			{
				eprintln!("debug: failed to read memory");
				return;
			}

			let string = String::from_utf8_lossy(&buffer);
			println!("[predicate {id}]: {string}");
		},
	);

	#[cfg(not(any(test, feature = "std")))]
	let debug = Func::wrap(
		store.as_context_mut(),
		|_: Caller<'_, &'e E>, _: u32, _: u32| {
			// noop
		},
	);

	<Linker<&'e E>>::new(engine)
		.define("env", "vrf_at", vrf_at)?
		.define("env", "time_at", time_at)?
		.define("env", "history_len", history_len)?
		.define("env", "block_number", block_number)?
		.define("env", "debug", debug)?
		.instantiate(store.as_context_mut(), module)?
		.start(store.as_context_mut())
}

const fn to_tuple(
	digest: &[u8; 32],
) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
	(
		u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]),
		u32::from_le_bytes([digest[4], digest[5], digest[6], digest[7]]),
		u32::from_le_bytes([digest[8], digest[9], digest[10], digest[11]]),
		u32::from_le_bytes([digest[12], digest[13], digest[14], digest[15]]),
		u32::from_le_bytes([digest[16], digest[17], digest[18], digest[19]]),
		u32::from_le_bytes([digest[20], digest[21], digest[22], digest[23]]),
		u32::from_le_bytes([digest[24], digest[25], digest[26], digest[27]]),
		u32::from_le_bytes([digest[28], digest[29], digest[30], digest[31]]),
	)
}
