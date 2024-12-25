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

impl Filter for Hot {
	fn matches(&self, data: &[u8]) -> bool {
		(self.fn_)(data)
	}
}

impl core::fmt::Debug for Hot {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("<runtime-func>").finish()
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

/// Matches any data without any condition.
#[derive(Clone)]
pub struct Anything;

impl IntoFilter<Hot, Anything> for Anything {
	fn into_filter(self) -> Hot {
		Hot {
			fn_: Rc::new(|_: &[u8]| true),
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
