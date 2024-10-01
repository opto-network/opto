use {
	proc_macro::{TokenStream, TokenTree},
	proc_macro2::{Literal, Span},
	quote::quote,
	std::collections::{HashMap, VecDeque},
	syn::parse_macro_input,
	valid::validate_predicate_signature,
};

mod gen;
mod valid;

#[proc_macro_attribute]
pub fn predicate(args: TokenStream, item: TokenStream) -> TokenStream {
	let attribs = parse_attributes(args);

	let item = parse_macro_input!(item as syn::ItemFn);
	validate_predicate_signature(&item);

	let id = attribs["id"].clone();
	let id_u32 = id.parse::<u32>().expect("Failed to parse id");
	let id_lit = Literal::u32_suffixed(id_u32);

	let item_fn_name = &item.sig.ident;
	let pred_id = format!("pred_{}", id);
	let const_name =
		syn::Ident::new(&format!("{item_fn_name}_id"), Span::call_site());

	let cfg_target = syn::Ident::new(pred_id.as_str(), Span::call_site());
	let context_expand = gen::predicate_context();

	let output = quote! {
		#item

		#[allow(non_upper_case_globals)]
		pub const #const_name: ::opto::PredicateId = ::opto::PredicateId(#id_u32);

		#[cfg(#cfg_target)]
		mod __abi_impl {

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

	kv_list.push(this_kv);
	kv_list.into_iter().map(parse_key_value).collect()
}

fn parse_key_value(kv: Vec<TokenTree>) -> (String, String) {
	let mut kv = kv.into_iter();
	let key = kv.next().unwrap();
	let val = kv.next().unwrap();

	(key.to_string(), val.to_string())
}
