use opto_core::*;

#[macro_export]
macro_rules! ensure {
	($e:expr) => {
		if !($e) {
			return false;
		}
	};
}

pub fn is_unlock(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.role, Role::Unlock(_, _))
}

pub fn is_policy(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.role, Role::Policy(_, _))
}

pub fn is_input(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Input)
}

pub fn is_output(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Output)
}

pub fn is_ephemeral(ctx: &Context<'_, impl Environment>) -> bool {
	matches!(ctx.location, Location::Ephemeral)
}

pub fn has_policy(
	ctx: &Context<'_, impl Environment>,
	id: &PredicateId,
) -> bool {
	ctx.object.policies.iter().any(|p| p.id == *id)
}

/// Tests whether policies on two objects are equivalent.
///
/// This means that they have the same policies with the same parameters,
/// regardless of order and ignores nonces.
#[cfg(feature = "stdpred")]
pub fn equivalent_policies(left: &Object, right: &Object) -> bool {
	use alloc::collections::BTreeSet;

	let left: BTreeSet<&Predicate> = left
		.policies
		.iter()
		.filter(|p| p.id != opto_stdpred::ids::NONCE)
		.collect();

	let right: BTreeSet<&Predicate> = right
		.policies
		.iter()
		.filter(|p| p.id != opto_stdpred::ids::NONCE)
		.collect();

	left == right
}

#[cfg(feature = "stdpred")]
pub fn uniqueness(ctx: &Context<'_, impl Environment>) -> Option<Digest> {
	ctx
		.object
		.policies
		.iter()
		.find(|p| p.id == opto_stdpred::ids::UNIQUE)
		.and_then(|p| p.params.as_slice().try_into().ok())
}

#[cfg(feature = "stdpred")]
pub fn uniqueness_equals(
	ctx: &Context<'_, impl Environment>,
	value: &Digest,
) -> bool {
	uniqueness(ctx).map_or(false, |u| u == *value)
}

pub fn is_only_policy_of_this_type(
	ctx: &Context<'_, impl Environment>,
) -> bool {
	ctx
		.object
		.policies
		.iter()
		.filter(|p| p.id == ctx.predicate_id())
		.count()
		== 1
}
