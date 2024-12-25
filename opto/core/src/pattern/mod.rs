use crate::{Object, Predicate};

mod cold;
mod hot;
mod object;
mod policies;
mod predicate;
mod transition;
mod unlock;

pub use {
	cold::Cold,
	hot::Hot,
	object::{ObjectPattern, ObjectsSetPattern},
	policies::PoliciesPattern,
	transition::TransitionPattern,
	unlock::UnlockPattern,
};

mod private {
	use core::marker::PhantomData;
	pub struct Sentinel<T>(PhantomData<fn() -> T>);
}

pub trait Filter: Clone + core::fmt::Debug {
	fn matches(&self, data: &[u8]) -> bool;
}

pub trait IntoFilter<F: Filter, X = ()> {
	fn into_filter(self) -> F;
}

/// A single named capture inside an object.
///
/// When adding patterns, they can be optionally named by using the `capture_*`
/// methods, in that case whenever a pattern matches, a reference to the item
/// (predicate, data, etc) that matched the pattern will be stored in the
/// `Capture` object.
#[derive(Clone, Debug, PartialEq)]
pub enum Capture<'a> {
	Policy(&'a Object, &'a Predicate, usize),
	Unlock(&'a Object, &'a Predicate, usize),
	Data(&'a Object),
}
