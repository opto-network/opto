use {
	super::*,
	opto_core::{Digest, Hashable},
	scale_info::prelude::*,
	sp_core::Get,
};

mod install;
mod wrap;

pub use {install::install, wrap::wrap};

fn produce_output<T: Config<I>, I: 'static>(
	object: Object<AtRest, Vec<u8>>,
	emit_event: bool,
) -> Result<Digest, Error<T, I>> {
	if object.encoded_size() > T::MaximumObjectSize::get() as usize {
		return Err(Error::<T, I>::ObjectTooLarge);
	}

	let digest = object.digest();
	let instance_count = Objects::<T, I>::get(digest)
		.map_or(0, |o| o.instance_count)
		.saturating_add(1);

	if emit_event {
		Pallet::<T, I>::deposit_event(Event::ObjectCreated {
			object: object.clone(),
		});
	}

	let stored_object = StoredObject {
		instance_count,
		object,
	};

	Objects::<T, I>::insert(digest, stored_object);

	Ok(digest)
}
