use {
	crate::{Object, Predicate},
	alloc::string::String,
};

mod cold;
mod hot;
mod object;
mod policies;
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

	pub trait Sealed {}
	pub struct Sentinel<T>(PhantomData<fn(T) -> T>);
}

pub trait Filter: Clone + private::Sealed {
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

/// A filter that matches bytestings on either predicates or object data.
///
/// Criteria can be optionally named, and when a pattern matches, the name and
/// the matched item will be accessible by the match name.
///
/// Criteria can be either hot or cold. Hot criteria are defined by a closure
/// and are compiled by Rust, they can't however be serialized to a persisted
/// form such as in a predicate param. They are more flixible than cold criteria
/// because they can contain arbirary Rust code. Hot criteria are often used by
/// solvers to match objects in search of intents they can solve.
///
/// Cold criteria are serializable and can be used in persisted forms, but they
/// are less flexible than hot criteria.
#[derive(Clone)]
pub struct DataCriterion<F: Filter> {
	capture: Option<String>,
	filter: F,
}

impl<F: Filter> DataCriterion<F> {
	fn matches(&self, data: &[u8]) -> bool {
		self.filter.matches(data)
	}
}
