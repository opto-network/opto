use {
	opto_core::{Digest, Expression, Predicate},
	subxt::utils::AccountId32,
};

pub trait ExpressionExt {
	fn signature(by: &AccountId32) -> Expression;
	fn preimage(preimage: &[u8]) -> Expression;
	fn constant(value: bool) -> Expression;
}

impl ExpressionExt for Expression {
	fn signature(by: &AccountId32) -> Expression {
		Predicate {
			id: opto_stdpred::ids::SR25519,
			params: by.0.to_vec(),
		}
		.into()
	}

	fn preimage(preimage: &[u8]) -> Expression {
		Predicate {
			id: opto_stdpred::ids::BLAKE2B_256,
			params: Digest::compute(preimage).to_vec(),
		}
		.into()
	}

	fn constant(value: bool) -> Expression {
		Predicate {
			id: opto_stdpred::ids::CONSTANT,
			params: vec![value as u8],
		}
		.into()
	}
}
