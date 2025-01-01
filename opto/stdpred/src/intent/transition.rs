use {
	crate::utils::is_unlock,
	opto_core::*,
	opto_onchain_macros::predicate,
	scale::Decode,
};

/// Predicate returns true if the state transition matches the
/// desired pattern.
///
/// This predicate is the foundational building block for intents.
#[predicate(id = 302, core_crate = opto_core)]
pub fn transition(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_unlock(&ctx));

	let Ok(expected_pattern) =
		opto_core::TransitionPattern::<Cold>::decode(&mut &params[..])
	else {
		return false;
	};

	expected_pattern.matches(&transition.clone().compact())
}
