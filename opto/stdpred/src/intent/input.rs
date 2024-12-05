use {
	crate::utils::is_unlock,
	opto_core::*,
	opto_onchain::predicate,
	scale::Decode,
};

/// Predicate returns true if the state transition contains some object.
/// The parameter is a SCALE encoded expected output object.
///
/// This predicate is the foundational building block for intents.
#[predicate(id = 302, core_crate = opto_core)]
pub fn input(
	ctx: Context<'_, impl Environment>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	ensure!(is_unlock(&ctx));

	let Ok(expected) = opto_core::Object::decode(&mut &params[..]) else {
		return false;
	};

	transition.inputs.iter().any(|object| *object == expected)
}
