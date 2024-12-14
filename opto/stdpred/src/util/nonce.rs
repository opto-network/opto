use {
	crate::utils::*,
	blake2::{digest::consts::U8, Digest},
	opto_core::*,
	opto_onchain_macros::predicate,
};

// u64 is 8 bytes
type Hasher = blake2::Blake2b<U8>;

#[predicate(id = 101, core_crate = opto_core)]
pub fn nonce(
	ctx: Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
	param: &[u8],
) -> bool {
	ensure!(is_policy(&ctx));
	ensure!(!is_ephemeral(&ctx));
	ensure!(param.len() == size_of::<u64>());
	ensure!(is_only_policy_of_this_type(&ctx));
	ensure!(!is_the_only_policy(&ctx));

	match ctx.location {
		Location::Input => true,
		Location::Output => {
			if is_first_output_nonce(&ctx, transition) {
				// as an optimization nonce value validity is only checked once by the
				// first output object that has a nonce policy, it will verify the
				// correctness of all other output objects that have a nonce policy.

				// use a small 64 bit hash size for nonce
				let mut hasher = Hasher::default();
				for input in transition.inputs.iter() {
					hasher.update(input.digest());
				}

				// each nonce in the output is expected to be:
				// H(inputs_hash_64 || output_index)
				let inputs_hash: [u8; 8] = hasher.finalize().into();
				for (ix, object) in transition.outputs.iter().enumerate() {
					if let Some(pred) =
						object.policies.iter().find(|p| p.id == ctx.predicate_id())
					{
						let Ok(u64_bytes) = pred.params.as_slice().try_into() else {
							return false;
						};

						let actual_nonce = u64::from_le_bytes(u64_bytes);

						let expected_nonce = hash_concat(&[
							&inputs_hash,
							(ix as u64).to_le_bytes().as_slice(),
							object.unlock.digest().as_slice(),
							&object.data,
						]);

						ensure!(actual_nonce == expected_nonce);
					}
				}
			}

			true
		}
		Location::Ephemeral => unreachable!("validated earlier"),
	}
}

fn is_first_output_nonce(
	ctx: &Context<'_, impl Environment>,
	transition: &Transition<Expanded>,
) -> bool {
	let object_index = ctx
		.object_index(transition)
		.expect("invalid predicate context");

	// check if there is any nonce policy in the previous objects
	let range = 0..object_index;
	for object in &transition.outputs[range] {
		if object.policies.iter().any(|p| p.id == ctx.predicate_id()) {
			return false;
		}
	}

	true
}

fn hash_concat(elems: &[&[u8]]) -> u64 {
	let mut hasher = Hasher::default();
	for elem in elems {
		hasher.update(elem);
	}
	u64::from_le_bytes(hasher.finalize().into())
}
