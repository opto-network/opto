use {
	quote::ToTokens,
	subxt_codegen::CodegenBuilder,
	syn::{parse2, File},
};

fn main() {
	let metadata: subxt_metadata::Metadata =
		opto_chain_runtime::runtime_metadata()
			.try_into()
			.expect("runtime metadata conversion failed");

	let mut generator = CodegenBuilder::new();
	generator.set_target_module(syn::parse_quote!(
		pub mod model {}
	));

	let mut code: File = parse2(
		generator
			.generate(metadata)
			.expect("metadata codegen failed"),
	)
	.expect("metadata codegen generated invalid code");

	swap_generated_opto_core(&mut code);

	let output_path = format!("{}/model.rs", std::env::var("OUT_DIR").unwrap());
	std::fs::write(output_path, code.to_token_stream().to_string()).unwrap();
}

/// Subxt code generator by default will try to generate all the code that is
/// already defined in `opto_core` crate off the runtime data. This function
/// will remove the opto_core generated code and redirect the code to use the
/// `opto_core` crate directly.
fn swap_generated_opto_core(code: &mut File) {
	fn visit(item: &mut syn::Item) {
		if let syn::Item::Mod(moditem) = item {
			let name = moditem.ident.to_string();

			if name == "opto_core" {
				// replace subtree with `pub use ::opto_core::*;`
				*item = syn::parse_quote! {
					pub use ::opto_core;
				};
				return;
			}

			if let Some((_, ref mut items)) = moditem.content {
				for item in items.iter_mut() {
					visit(item);
				}
			}
		}
	}

	for item in code.items.iter_mut() {
		visit(item);
	}
}
