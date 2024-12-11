#![allow(dead_code)]

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

pub fn is_the_only_policy(ctx: &Context<'_, impl Environment>) -> bool {
	is_policy(ctx) && ctx.object.policies.len() == 1
}
