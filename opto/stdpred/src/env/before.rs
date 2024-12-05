use {
	core::time::Duration,
	opto_core::*,
	opto_onchain::predicate,
	scale::Decode,
};

/// Predicate that checks if the current time is before a given timestamp.
///
/// param: u32 - timestamp in seconds since epoch
#[predicate(id = 400, core_crate = opto_core)]
pub fn before_time(
	ctx: Context<'_, impl Environment>,
	_: &Transition,
	params: &[u8],
) -> bool {
	let Ok(timestamp) = u32::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.time_now() < Duration::from_secs(timestamp as u64)
}

#[predicate(id = 401, core_crate = opto_core)]
pub fn before_block(
	ctx: Context<'_, impl Environment>,
	_: &Transition,
	params: &[u8],
) -> bool {
	let Ok(block) = u32::decode(&mut &params[..]) else {
		return false;
	};

	ctx.env.block_number() < block
}
