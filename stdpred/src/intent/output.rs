use {
	crate::utils::is_unlock,
	opto::{Context, Transition},
	scale::Decode,
};

/// Predicate returns true if the state transition contains some object.
/// The parameter is a SCALE encoded expected output object.
///
/// This predicate is the foundational building block for intents.
#[opto::predicate(id = 300)]
pub fn output(
	ctx: Context<'_>,
	transition: &Transition,
	params: &[u8],
) -> bool {
	ensure!(is_unlock(&ctx));

	let Ok(expected) = opto::Object::decode(&mut &params[..]) else {
		return false;
	};

	transition.outputs.iter().any(|output| *output == expected)
}
