#![allow(static_mut_refs)]

use {
	proc_macro::{TokenStream, TokenTree},
	proc_macro2::{Literal, Span},
	quote::quote,
	std::collections::{BTreeMap, HashMap, VecDeque},
	syn::parse_macro_input,
	valid::validate_predicate_signature,
};

mod gen;
mod valid;

static mut INDEX: BTreeMap<&'static str, BTreeMap<String, u32>> =
	BTreeMap::new();

#[proc_macro_attribute]
pub fn predicate(args: TokenStream, item: TokenStream) -> TokenStream {
	let attribs = parse_attributes(args);

	let item = parse_macro_input!(item as syn::ItemFn);
	validate_predicate_signature(&item);

	let id = attribs["id"].clone();
	let id_u32 = id.parse::<u32>().expect("Failed to parse id");
	let id_lit = Literal::u32_suffixed(id_u32);

	let crate_name = attribs
		.get("core_crate")
		.map(|s| s.as_str())
		.unwrap_or("opto");

	let crate_name = syn::Ident::new(crate_name, Span::call_site());

	let item_fn_name = &item.sig.ident;
	let pred_id = format!("pred_{}", id);

	let cfg_target = syn::Ident::new(pred_id.as_str(), Span::call_site());
	let context_expand = gen::predicate_context(&crate_name);

	let env_code = include_str!("env.rs.txt")
		.replace("#core_crate", crate_name.to_string().as_str());
	let env_code: proc_macro2::TokenStream = env_code.parse().unwrap();

	// keep track of all registered predicates
	unsafe {
		INDEX
			.entry(env!("CARGO_CRATE_NAME"))
			.or_default()
			.insert(item_fn_name.to_string().to_uppercase(), id_u32)
	};

	let output = quote! {
		#item

		#[cfg(#cfg_target)]
		mod __abi_impl {

			#env_code

			#[export_name="_pred_id"]
			static PRED_ID: u32 = #id_lit;

			#[export_name="_eval"]
			extern "C" fn eval(
				location: u32,
				object_index: u32,
				pred_role: u32,
				pred_index: u32,
				transition_ptr: *const u8,
				transition_len: u32,
				params_ptr: *const u8,
				params_len: u32) -> u32 {

					#context_expand

					match super::#item_fn_name(ctx, &transition, params) {
						true => 1,
						false => 0,
					}
			}
		}
	};

	output.into()
}

#[proc_macro]
pub fn predicates_index(args: TokenStream) -> TokenStream {
	let attribs = parse_attributes(args);
	let crate_name = attribs
		.get("core_crate")
		.map(|s| s.as_str())
		.unwrap_or("opto");
	let crate_name = syn::Ident::new(crate_name, Span::call_site());

	let mut output = vec![];
	for (name, id) in
		unsafe { INDEX.entry(env!("CARGO_CRATE_NAME")).or_default().iter() }
	{
		let name = syn::Ident::new(name, Span::call_site());
		let id_lit = Literal::u32_suffixed(*id);
		let item = quote! {
			pub const #name: #crate_name::PredicateId =
				#crate_name::PredicateId(#id_lit);
		};
		output.push(item);
	}

	quote! {
		#(#output)*
	}
	.into()
}

fn parse_attributes(args: TokenStream) -> HashMap<String, String> {
	let mut remaining: VecDeque<_> = args.into_iter().collect();
	let mut kv_list = Vec::new();

	let mut this_kv = Vec::new();
	while let Some(item) = remaining.pop_front() {
		if let TokenTree::Punct(p) = item {
			if p.as_char() == ',' {
				kv_list.push(this_kv);
				this_kv = Vec::new();
			}
		} else {
			this_kv.push(item);
		}
	}

	if !this_kv.is_empty() {
		kv_list.push(this_kv);
	}
	kv_list.into_iter().filter_map(parse_key_value).collect()
}

fn parse_key_value(kv: Vec<TokenTree>) -> Option<(String, String)> {
	let mut kv = kv.into_iter();
	let key = kv.next()?;
	let val = kv.next()?;

	Some((key.to_string(), val.to_string()))
}
