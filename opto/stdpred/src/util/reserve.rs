use {
	crate::{
		utils::{
			is_ephemeral,
			is_only_policy_of_this_type,
			is_policy,
			is_the_only_policy,
		},
		AccountId,
	},
	opto_core::*,
	opto_onchain_macros::predicate,
	scale::{Decode, Encode},
	scale_info::TypeInfo,
};

pub const ONE_MINUTE: u32 = 60;
pub const ONE_HOUR: u32 = 60 * ONE_MINUTE;
pub const ONE_DAY: u32 = 24 * ONE_HOUR;
pub const ONE_WEEK: u32 = 7 * ONE_DAY;

#[derive(Debug, Clone, PartialEq, Encode, Decode, TypeInfo)]
pub struct Reservation {
	/// The balance in the native token that is needed as a collateral for the
	/// reservation.
	pub deposit: u64,

	/// The account that receives the collateral if the reservation is not
	/// consumed before the reservation period has expired.
	pub payee: AccountId,

	/// The duration of the reservation
	pub duration: core::time::Duration,

	/// The maximum timestamp when a reservation can be made.
	/// After this timestamp the object is not reservable.
	///
	/// if the reservation is made before the timestamp, the reservation is
	/// automatically cancelled after this timestamp if set.
	///
	/// timestamp is duration elapsed since 1970-01-01 00:00:00 UTC.
	pub not_after: Option<core::time::Duration>,

	/// The minimum timestamp when a reservation can be made.
	/// Before this timestamp the object is not reservable.
	/// timestamp is duration elapsed since 1970-01-01 00:00:00 UTC.
	pub not_before: Option<core::time::Duration>,
}

/// Reserve policy.
///
/// This policy is used when an object may be reservable by certain accounts for
/// a certain period with a collateral in native token. It is used in situations
/// when there is an intent with a reward for a certain future state and an
/// account wishes to reserve the right to consume that reward in the future
/// without having a solution to the intent at the moment.
///
/// When an object is reserved it can can only be consumed by a transition
/// applied by the account that reserved it for the duration of the reservation.
/// All reservations are time bound and the object is automatically unreserved
/// after the reservation period has expired.
///
/// The account reserving the object must provide a collateral in native token
/// in the amount specified in the reservation policy. The collateral is locked
/// and returned to the account when the reserved object is consumed before the
/// reservation period has expired. If the object is not consumed before the
/// reservation period has expired, the collateral is forfeited and transfered
/// to the account specified in the reservation policy and the reservation is
/// automatically cancelled.
#[predicate(id = 103, core_crate = opto_core)]
pub fn reserve(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(!is_the_only_policy(&ctx));

	let Ok(param) = Reservation::decode(&mut &params[..]) else {
		return false;
	};

	ensure!(param.deposit >= ctx.env.minimum_reservation_deposit());
	ensure!(param.duration >= ctx.env.minimum_reservation_duration());

	if let (Some(not_after), Some(not_before)) =
		(param.not_after, param.not_before)
	{
		ensure!(not_after > not_before);
	}

	true
}
