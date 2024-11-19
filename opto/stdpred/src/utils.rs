#![allow(dead_code)]

use opto_core::eval::{Context, Location, Role};

#[macro_export]
macro_rules! ensure {
	($e:expr) => {
		if !($e) {
			return false;
		}
	};
}

pub fn is_unlock(ctx: &Context) -> bool {
	matches!(ctx.role, Role::Unlock(_, _))
}

pub fn is_policy(ctx: &Context) -> bool {
	matches!(ctx.role, Role::Policy(_, _))
}

pub fn is_input(ctx: &Context) -> bool {
	matches!(ctx.location, Location::Input)
}

pub fn is_output(ctx: &Context) -> bool {
	matches!(ctx.location, Location::Output)
}

pub fn is_ephemeral(ctx: &Context) -> bool {
	matches!(ctx.location, Location::Ephemeral)
}

pub fn is_only_policy_of_this_type(ctx: &Context) -> bool {
	ctx
		.object
		.policies
		.iter()
		.filter(|p| p.id == ctx.predicate_id())
		.count()
		== 1
}
