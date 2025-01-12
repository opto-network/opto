use {
	super::*,
	crate::interface::Balance,
	frame::{prelude::*, traits::ReservableCurrency},
	model::ActiveObject,
	opto_core::{Digest, Hashable},
	scale_info::prelude::*,
	sp_core::Get,
	sp_runtime::SaturatedConversion,
	stdpred::util::reserve::Reservation,
};

mod apply;
mod hooks;
mod install;
mod reserve;
mod unwrap;
mod wrap;

pub use {
	apply::apply,
	hooks::{reclaim_expired_reservations, timestamp_update, vrf_update},
	install::install,
	reserve::reserve,
	unwrap::unwrap,
	wrap::wrap,
};

fn consume_input<T: Config<I>, I: 'static>(
	digest: Digest,
	consumer: &T::AccountId,
) -> Result<Object<Predicate, Vec<u8>>, Error<T, I>> {
	let mut active_object = Objects::<T, I>::get(digest) //
		.ok_or(Error::<T, I>::InputObjectNotFound)?;

	// if the object is found, try to release a reservation
	// placed by the consumer (if found).
	try_release_reservation::<T, I>(&mut active_object, consumer);

	// the number of reservations is equal to the number of
	// instances, then it means that none of th object copies are
	// available for consumption.
	if active_object.reservations.len() >= active_object.instance_count as usize {
		return Err(Error::<T, I>::InputObjectReserved);
	}

	// if the object is found, we need to decrement the
	// remaining field and check if it is zero. If it is
	// zero, we need to remove the object from storage.
	let remaining = active_object.instance_count.saturating_sub(1);

	if remaining == 0 {
		Objects::<T, I>::remove(digest);
	} else {
		Objects::<T, I>::mutate_extant(digest, |o| {
			o.instance_count = remaining;
			o.reservations = active_object.reservations;
		});
	}

	// if the object has a uniqueness policy, we need to remove it
	// and allow new objects with the same uniqueness policy to be
	// created.
	if let Some(uniqueness) = uniqueness::<T, I>(&active_object.content) {
		Uniques::<T, I>::remove(uniqueness);
	}

	Ok(active_object.content)
}

fn produce_output<T: Config<I>, I: 'static>(
	object: Object<Predicate, Vec<u8>>,
) -> Result<Digest, Error<T, I>> {
	if object.encoded_size() > T::MaximumObjectSize::get() as usize {
		return Err(Error::<T, I>::ObjectTooLarge);
	}

	if object.policies.len() > T::MaximumObjectPolicies::get() as usize {
		return Err(Error::<T, I>::TooManyPolicies);
	}

	let digest = object.digest();

	// uphold the uniqueness policy
	if let Some(uniqueness) = uniqueness::<T, I>(&object) {
		if Uniques::<T, I>::contains_key(uniqueness) {
			return Err(Error::<T, I>::UniqueAlreadyExists);
		}

		Uniques::<T, I>::insert(uniqueness, digest);
	}

	let (instance_count, reservations) = Objects::<T, I>::get(digest)
		.map(|o| (o.instance_count, o.reservations))
		.unwrap_or_default();

	let active_object = ActiveObject {
		instance_count: instance_count.saturating_add(1),
		reservations,
		content: object,
	};

	Objects::<T, I>::insert(digest, active_object);

	Ok(digest)
}

/// Checks if an object is tagged with a uniqueness policy and returns the
/// digest of the policy if it is.
fn uniqueness<T: Config<I>, I: 'static>(object: &Object) -> Option<Digest> {
	let policy = object
		.policies
		.iter()
		.find(|p| p.id == T::UniquePolicyPredicate::get())?;

	policy.params.as_slice().try_into().ok()
}

fn try_release_reservation<T: Config<I>, I: 'static>(
	object: &mut ActiveObject,
	consumer: &T::AccountId,
) {
	// an object with this digest is found. We need to check
	// the state of the reservations on this object. If the consumer
	// is having a hold on the object, then we need to remove the
	// reservation with the lowest duration for this consumer.
	if !object.reservations.is_empty() {
		let mut reservation_index = None;
		let mut min_duration = super::model::Timestamp::MAX;

		for (i, reservation) in object.reservations.iter().enumerate() {
			if reservation.by == *consumer && reservation.until < min_duration {
				min_duration = reservation.until;
				reservation_index = Some(i);
			}
		}

		if let Some(index) = reservation_index {
			// there was a reservation that was released, removed it from
			// the list of reservations and refund the deposit.
			let hold = object.reservations.remove(index);

			// refund the deposit
			let deposit: Balance = object
				.content
				.policies
				.iter()
				.find(|p| p.id == stdpred::ids::RESERVE)
				.and_then(|p| Reservation::decode(&mut p.params.as_slice()).ok())
				.map(|params: Reservation| params.deposit)
				.expect(
					"a reservation was placed on an object without reservation policy \
					 attached",
				);

			// Return the deposit to the consumer
			<T as super::Config<I>>::Currency::unreserve(
				consumer,
				deposit.saturated_into(),
			);

			// remove the reservation expiration entry. Note that an object may be
			// reserved multiple times by the same account with the same expiration
			// time, because there may be more than one copy of the same object.
			// Here we are going to remove only one occurrence of the digest from
			// the expiration list.
			let object_digest = object.content.digest();
			ReservationExpirations::<T, I>::mutate(hold.until, |entry| {
				if let Some(index) = entry.iter().position(|d| *d == object_digest) {
					entry.remove(index);
				}
			});

			// emit an event that the reservation was released and the object is
			// consumed and not available anymore.
			Pallet::<T, I>::deposit_event(Event::<T, I>::ReservationReleased {
				object: object.content.digest(),
				by: consumer.clone(),
				consumed: true,
			});
		}
	}
}
