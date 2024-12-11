use {
	crate::utils::{
		is_ephemeral,
		is_only_policy_of_this_type,
		is_policy,
		is_the_only_policy,
	},
	opto_core::*,
	opto_onchain_macros::predicate,
};

/// Unique
///
/// Used when we need to guarantee uniqueness of an object. At any point in time
/// the runtime will only allow one object with this policy params to exist in
/// the state. It acts as a tag that can be attached to any object. The object
/// itself can morph through state transistions but no two objects with the same
/// unique policy params can exist at the same time.
///
/// This predicate can only be used as a policy on an object. This is an
/// intrinsic predicate and has special meaning to the runtime. It is installed
/// at genesis as part of stdpred.
#[predicate(id = 102, core_crate = opto_core)]
pub fn unique(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(params.len() == Digest::SIZE);
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(!is_the_only_policy(&ctx));

	true
}
