use {proc_macro2::TokenStream, quote::quote};

pub fn predicate_context(core_crate: &syn::Ident) -> TokenStream {
	quote! {
		use ::#core_crate::{
			Object, Transition, Predicate, Expanded,
			eval::{Location, Context, Role},
			codec::Decode,
		};

		let mut transition = unsafe {
			::core::slice::from_raw_parts(transition_ptr, transition_len as usize)
		};

		let transition: Transition<Expanded> = Decode::decode(&mut transition)
			.expect("Failed to decode transition bytes");

		let location = match location {
				0 => Location::Input,
				1 => Location::Ephemeral,
				2 => Location::Output,
				_ => panic!("Invalid location value"),
		};

		let index = object_index as usize;
		let object: &Object = match location {
			Location::Input => &transition.inputs[index],
			Location::Ephemeral => &transition.ephemerals[index],
			Location::Output => &transition.outputs[index],
		};

		let index = pred_index as usize;
		let role: Role<'_, Predicate> = match pred_role {
			0 => Role::Policy(&object.policies[index], index),
			1 => Role::Unlock(&object.unlock.as_ops()[index].as_predicate().unwrap(), index),
			_ => panic!("Invalid role value"),
		};

		let params = unsafe {
			::core::slice::from_raw_parts(params_ptr, params_len as usize)
		};

		let ctx = Context {
			role,
			object,
			location,
			env: &PredicateEnvironment,
		};
	}
}
