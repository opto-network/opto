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
	opto_core::*,
	opto_onchain_macros::predicate,
	repr::AsObject,
	scale::Decode,
};

#[predicate(id = 1000, core_crate = opto_core)]
pub fn coin(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(u32::decode(&mut &params[..])
		.map(|asset_id| asset_id != 0)
		.unwrap_or(false));
	ensure!(u64::decode(&mut ctx.object.data.as_slice())
		.map(|balance| balance != 0)
		.unwrap_or(false));

	if is_input(&ctx) {
		// if the coin is an input, then it should be valid because it must have
		// passed all the checks in the previous state when it was an output.
		return true;
	}

	let Ok(input_balance) = total_balance(&ctx, &transition.inputs, params)
	else {
		return false;
	};

	let Ok(output_balance) = total_balance(&ctx, &transition.outputs, params)
	else {
		return false;
	};

	// no new coins were created in this transition
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
	ctx: &Context<'_, impl Environment>,
	set: &[AsObject<Expanded>],
	coinid: &[u8],
) -> Result<u128, Error> {
	set
		.iter()
		.filter(|obj| {
			obj
				.policies
				.iter()
				.any(|p| p.id == ctx.predicate_id() && p.params == coinid)
		})
		.try_fold(0u128, |acc, object| {
			acc
				.checked_add(
					u64::decode(&mut object.data.as_slice()).map_err(|_| Error::Decode)?
						as u128,
				)
				.ok_or(Error::ValueOverflow)
		})
}
