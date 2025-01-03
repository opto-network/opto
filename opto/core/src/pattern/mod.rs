mod cold;
mod data;
mod hot;
mod object;
mod policies;
mod predicate;
mod transition;
mod unlock;

pub use {
	cold::{Cold, ColdCaptureExt, Comparable, Comparison, Condition},
	data::{DataPattern, IntoDataPattern},
	hot::Hot,
	object::{Capture, ObjectCapture, ObjectPattern, ObjectsSetPattern},
	policies::PoliciesPattern,
	predicate::{PredicateIdExt, PredicatePattern},
	transition::{TransitionCapture, TransitionPattern},
	unlock::UnlockPattern,
};

mod private {
	use core::marker::PhantomData;
	pub struct Sentinel<T>(PhantomData<fn() -> T>);
	pub trait Sealed {}
}

pub trait Filter: Clone + core::fmt::Debug + private::Sealed {
	fn any() -> Self;
	fn matches(&self, data: &[u8]) -> bool;
}

pub trait IntoFilter<F: Filter, X = ()> {
	fn into_filter(self) -> F;
}

/// Matches any data without any condition.
#[derive(Clone)]
pub struct Anything;
