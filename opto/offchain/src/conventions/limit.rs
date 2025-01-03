use {
	core::time::Duration,
	opto_core::*,
	scale::{Decode, Encode},
};

#[derive(Debug, Clone, Encode, Decode)]
pub enum Error {
	NotALimitOrder,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct LimitOrder;

impl TryFrom<Object> for LimitOrder {
	type Error = Error;

	fn try_from(_object: Object) -> Result<Self, Self::Error> {
		todo!()
	}
}

impl LimitOrder {
	pub fn expires_in(self, _duration: Duration) -> Self {
		todo!()
	}
}
