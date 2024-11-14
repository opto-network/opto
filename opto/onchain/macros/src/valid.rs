pub fn validate_predicate_signature(item: &syn::ItemFn) {
	let sig = &item.sig;
	if sig.inputs.len() != 3 {
		panic!("Expected 3 arguments, found {}", sig.inputs.len());
	}

	validate_context(&sig.inputs[0]);
	validate_transition(&sig.inputs[1]);
	validate_params(&sig.inputs[2]);
	validate_return_type(sig);
}

fn validate_context(arg: &syn::FnArg) {
	if let syn::FnArg::Typed(ctx) = arg {
		if let syn::Type::Path(ctx) = &*ctx.ty {
			if let Some(last) = ctx.path.segments.last() {
				if last.ident == "Context" {
					return;
				}
			}
		}
	}
	panic!("Expected Context as first argument");
}

fn validate_transition(arg: &syn::FnArg) {
	if let syn::FnArg::Typed(ctx) = arg {
		if let syn::Type::Reference(ctx) = &*ctx.ty {
			if let syn::Type::Path(ctx) = &*ctx.elem {
				if let Some(last) = ctx.path.segments.last() {
					if last.ident == "Transition" {
						return;
					}
				}
			}
		}
	}
	panic!("Expected Transition as second argument");
}

fn validate_params(arg: &syn::FnArg) {
	if let syn::FnArg::Typed(ctx) = arg {
		if let syn::Type::Reference(ctx) = &*ctx.ty {
			if let syn::Type::Slice(ctx) = &*ctx.elem {
				if let syn::Type::Path(ctx) = &*ctx.elem {
					if ctx.path.is_ident("u8") {
						return;
					}
				}
			}
		}
	}
	panic!("Expected &[u8] as third argument");
}

fn validate_return_type(sig: &syn::Signature) {
	if let syn::ReturnType::Type(_, ty) = &sig.output {
		if let syn::Type::Path(ty) = &**ty {
			if let Some(last) = ty.path.segments.last() {
				if last.ident == "bool" {
					return;
				}
			}
		}
	}
	panic!("Expected bool return type");
}
