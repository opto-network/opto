//! Coin Predicate
//!
//! This predicate governs the behavior of a fungible token. The following are
//! the rules that the predicate enforces:
//!
//! - The predicate can only be used in the policy of an object.
//! - The policy cannot be attached to an ephemeral object.
//! - Coins are identified by a 32-bit unsigned integer that maps to asset id
//!   from `pallet_assets``.
//! - Coins cannot be minted in a state transition. Coins are only minted by
//!   `pallet_objects` when an asset is wrapped.
//! - The amount of coins is stored in the data section of the object that
//!   carries the `coin` policy.
//! - The amount of coins is a 64-bit unsigned integer encoded in scale format.
//! - A coin object cannot carry zero coins.
//! - The amount of coins in the input objects must be greater than or equal to
//!   the amount of coins in the output objects.

use {
	crate::{ensure, utils::*},
	opto_core::{
		eval::Context,
		repr::{AsObject, Expanded},
		Transition,
	},
	scale::Decode,
};

#[opto_onchain::predicate(id = 1000, core_crate = opto_core)]
pub fn coin(ctx: Context<'_>, transition: &Transition, params: &[u8]) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!((1..=16).contains(&params.len()));
	ensure!(u64::decode(&mut ctx.object.data.as_slice()).is_ok());

	let Ok(input_balance) = total_balance(&ctx, &transition.inputs, params)
	else {
		return false;
	};

	let Ok(output_balance) = total_balance(&ctx, &transition.outputs, params)
	else {
		return false;
	};

	ensure!(input_balance >= output_balance);

	true
}

enum Error {
	Decode,
	ValueOverflow,
}

/// Calculates the total balance of coin objects of a given coin id
/// on a given set of objects. This is used to sum up all input coins
/// or all output coins in a state transition.
fn total_balance(
	ctx: &Context<'_>,
	set: &[AsObject<Expanded>],
	coinid: &[u8],
) -> Result<u64, Error> {
	set
		.iter()
		.filter(|obj| {
			obj
				.policies
				.iter()
				.any(|p| p.id == ctx.predicate_id() && p.params == coinid)
		})
		.try_fold(0u64, |acc, object| {
			acc
				.checked_add(
					u64::decode(&mut object.data.as_slice())
						.map_err(|_| Error::Decode)?,
				)
				.ok_or(Error::ValueOverflow)
		})
}
