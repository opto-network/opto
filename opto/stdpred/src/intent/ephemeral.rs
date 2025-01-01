use {
	crate::utils::is_unlock,
	opto_core::*,
	opto_onchain_macros::predicate,
	scale::Decode,
};

/// Predicate returns true if the state transition contains some ephemeral
/// objects.
///
/// This predicate is the foundational building block for intents.
#[predicate(id = 301, core_crate = opto_core)]
pub fn ephemeral(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_unlock(&ctx));

	let Ok(expected_pattern) =
		opto_core::ObjectsSetPattern::<Cold>::decode(&mut &params[..])
	else {
		return false;
	};

	expected_pattern.matches(&transition.ephemerals)
}
