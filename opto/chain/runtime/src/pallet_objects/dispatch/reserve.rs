use {
	super::*,
	core::time::Duration,
	frame::traits::UnixTime,
	model::Hold,
	stdpred::util::reserve::Reservation,
};

pub fn reserve<T: Config<I> + pallet_assets::Config<I>, I: 'static>(
	origin: OriginFor<T>,
	digest: Digest,
) -> DispatchResult {
	let account = ensure_signed(origin)?;

	let object = Objects::<T, I>::get(digest) //
		.ok_or(Error::<T, I>::InputObjectNotFound)?;

	// check if there are any unreserved instances left
	if object.reservations.len() >= object.instance_count as usize {
		return Err(Error::<T, I>::InputObjectReserved.into());
	}

	// An object must be explicitly marked as reservable
	// by attaching a reservation policy to it for it to be
	// reservable.
	let policy = extract_policy::<T, I>(&object.content)?;

	// find out the expiration time of the reservation
	let expiration = calculate_exipration::<T, I>(&policy)?;

	// reserve the deposit for this object from the account
	// native token balance.
	<T as super::Config<I>>::Currency::reserve(
		&account,
		policy.deposit.saturated_into(),
	)?;

	// add the reservation to the object
	Objects::<T, I>::mutate_extant(digest, |object| {
		object.reservations.push(Hold {
			by: account.clone(),
			until: expiration.as_secs(),
		});
	});

	// register the reservation expiration rounded to the neareset second
	ReservationExpirations::<T, I>::mutate(expiration.as_secs(), |entry| {
		entry.push(digest);
	});

	// emit an event that the object has been reserved and is not
	// available for consumption by any other account until the
	// expiration time.
	Pallet::<T, I>::deposit_event(Event::<T, I>::ObjectReserved {
		object: digest,
		by: account,
		until: expiration.as_secs(),
	});

	Ok(())
}

fn extract_policy<T, I>(object: &Object) -> Result<Reservation, Error<T, I>> {
	let policy = object
		.policies
		.iter()
		.find(|p| p.id == stdpred::ids::RESERVE)
		.ok_or(Error::<T, I>::ReservationNotAllowed)?;

	Reservation::decode(&mut policy.params.as_slice())
		.map_err(|_| Error::<T, I>::InvalidReservationParameters)
}

fn calculate_exipration<T: Config<I>, I: 'static>(
	reservation_policy: &Reservation,
) -> Result<Duration, Error<T, I>> {
	let current_timestamp = pallet_timestamp::Pallet::<T>::now();

	// ensure that we are within the allowed reservation period
	if let Some(not_before) = reservation_policy.not_before {
		ensure!(
			current_timestamp >= not_before,
			Error::<T, I>::ReservationNotAllowedYet
		);
	}

	if let Some(not_after) = reservation_policy.not_after {
		ensure!(
			current_timestamp <= not_after,
			Error::<T, I>::ReservationNotAllowedAnymore
		);
	}

	let mut expiration = current_timestamp + reservation_policy.duration;

	// if there is a not_after timestamp, we need to ensure
	// that the reservation is capped at that timestamp
	if let Some(not_after) = reservation_policy.not_after {
		expiration = expiration.min(not_after);
	}

	Ok(expiration)
}
