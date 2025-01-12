use {
	core::time::Duration,
	opto_core::*,
	opto_onchain_macros::predicate,
	scale::Decode,
};

/// Predicate that checks if the current time is before a given timestamp.
///
/// param: u32 - timestamp in milliseconds since epoch
#[predicate(id = 400, core_crate = opto_core)]
pub fn before_time(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let Ok(timestamp) = Duration::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.time_now() < timestamp
}

#[predicate(id = 401, core_crate = opto_core)]
pub fn before_block(
	ctx: Context<'_, impl Environment>,
	_: &Transition<Expanded>,
	params: &[u8],
) -> bool {
	let Ok(block) = u32::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.block_number() < block
}
