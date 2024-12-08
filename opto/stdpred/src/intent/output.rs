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
#[predicate(id = 300, core_crate = opto_core)]
pub fn output(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	ensure!(is_unlock(&ctx));

	let Ok(expected) = opto_core::Object::decode(&mut &params[..]) else {
		return false;
	};

	transition.outputs.iter().any(|output| *output == expected)
}
