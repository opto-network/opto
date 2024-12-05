use {
	super::*,
	opto_core::{Digest, Hashable},
	scale_info::prelude::*,
	sp_core::Get,
};

mod apply;
mod init;
mod install;
mod unwrap;
mod wrap;

pub use {
	apply::apply,
	init::{timestamp_update, vrf_update},
	install::install,
	unwrap::unwrap,
	wrap::wrap,
};

fn consume_input<T: Config<I>, I: 'static>(
	digest: Digest,
) -> Result<Object<AtRest, Vec<u8>>, Error<T, I>> {
	let stored_object = Objects::<T, I>::get(digest) //
		.ok_or(Error::<T, I>::InputObjectNotFound)?;

	// if the object is found, we need to decrement the
	// remaining field and check if it is zero. If it is
	// zero, we need to remove the object from storage.
	let remaining = stored_object.instance_count.saturating_sub(1);

	if remaining == 0 {
		Objects::<T, I>::remove(digest);
	} else {
		Objects::<T, I>::insert(digest, StoredObject {
			instance_count: remaining,
			object: stored_object.object.clone(),
		});
	}

	Ok(stored_object.object)
}

fn produce_output<T: Config<I>, I: 'static>(
	object: Object<AtRest, Vec<u8>>,
) -> Result<Digest, Error<T, I>> {
	if object.encoded_size() > T::MaximumObjectSize::get() as usize {
		return Err(Error::<T, I>::ObjectTooLarge);
	}

	if object.policies.len() > T::MaximumObjectPolicies::get() as usize {
		return Err(Error::<T, I>::TooManyPolicies);
	}

	let digest = object.digest();
	let instance_count = Objects::<T, I>::get(digest)
		.map_or(0, |o| o.instance_count)
		.saturating_add(1);

	let stored_object = StoredObject {
		instance_count,
		object,
	};

	Objects::<T, I>::insert(digest, stored_object);

	Ok(digest)
}
