use {
	super::{private, Filter, IntoFilter, PoliciesPattern},
	crate::PredicateId,
	alloc::rc::Rc,
	scale::Decode,
};

type HotCaptureFn = Rc<dyn Fn(&[u8]) -> bool>;

#[derive(Clone)]
pub struct Hot {
	fn_: HotCaptureFn,
}

impl private::Sealed for Hot {}

impl Filter for Hot {
	fn matches(&self, data: &[u8]) -> bool {
		(self.fn_)(data)
	}
}

impl<T, C> IntoFilter<Hot, T> for C
where
	C: Fn(T) -> bool + 'static,
	T: Decode + 'static,
{
	fn into_filter(self) -> Hot {
		Hot {
			fn_: Rc::new(move |value: &[u8]| {
				let Ok(value) = T::decode(&mut &value[..]) else {
					return false;
				};

				self(value)
			}),
		}
	}
}

impl<C> IntoFilter<Hot, private::Sentinel<()>> for C
where
	C: Fn(&[u8]) -> bool + 'static,
{
	fn into_filter(self) -> Hot {
		Hot {
			fn_: Rc::new(move |value: &[u8]| self(value)),
		}
	}
}

impl PoliciesPattern<Hot> {
	/// Creates a new hot policies pattern.
	/// Those patterns cannot be serialized and stored.
	pub fn hot() -> PoliciesPattern<Hot> {
		PoliciesPattern {
			required: Default::default(),
			optional: Default::default(),
			exact: false,
		}
	}

	pub fn must_include(self, policy: PredicateId) -> Self {
		self.must_match(policy, |_: ()| true)
	}

	pub fn may_include(self, policy: PredicateId) -> Self {
		self.may_match(policy, |_: ()| true)
	}
}
