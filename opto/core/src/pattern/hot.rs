use {
	super::{private, Anything, Filter, IntoFilter},
	crate::codec::Decode,
	alloc::rc::Rc,
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

	fn any() -> Self {
		Hot {
			fn_: Rc::new(|_: &[u8]| true),
		}
	}
}

impl core::fmt::Debug for Hot {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
