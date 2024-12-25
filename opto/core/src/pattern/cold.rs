use super::{Filter, PoliciesPattern};

#[derive(Clone, Debug)]
pub struct Cold;

impl Filter for Cold {
	fn matches(&self, _data: &[u8]) -> bool {
		false
	}
}

// Instantiation
impl PoliciesPattern<Cold> {
	/// Creates a new cold policies pattern.
	/// Those patterns can be serialized and stored.
	pub fn cold() -> PoliciesPattern<Cold> {
		PoliciesPattern {
			required: Default::default(),
			optional: Default::default(),
			exact: false,
		}
	}
}
