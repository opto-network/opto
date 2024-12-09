use {
	crate::utils::{is_ephemeral, is_policy},
	opto_core::*,
	opto_onchain::predicate,
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

	// there can be only one unique tag attached to an object at a time
	ctx
		.object
		.policies
		.iter()
		.filter(|p| p.id == ctx.predicate_id())
		.count()
		== 1
}
