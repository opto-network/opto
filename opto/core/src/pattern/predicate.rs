use {
	super::{Filter, IntoFilter},
	crate::{Predicate, PredicateId},
};

/// A filter that matches predicates.
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
#[derive(Clone, Debug)]
pub enum PredicatePattern<F: Filter> {
	Any,
	Criteria {
		id: PredicateId,
		filter: F,
		capture: Option<String>,
	},
}

impl<F: Filter> PredicatePattern<F> {
	/// Wildcard pattern that matches any predicate
	/// cannot be captured.
	pub fn any() -> Self {
		Self::Any
	}

	/// Create new unnamed predicate pattern. This will be used to match a
	/// predicate without capturing it.
	pub fn new<T>(id: PredicateId, filter: impl IntoFilter<F, T>) -> Self {
		Self::Criteria {
			id,
			filter: filter.into_filter(),
			capture: None,
		}
	}

	/// Create new named predicate pattern.
	///
	/// If this pattern matches a predicate, then the matched predicate will be
	/// accessible by the given name in the capture set.
	pub fn named<T>(
		id: PredicateId,
		filter: impl IntoFilter<F, T>,
		capture: impl AsRef<str>,
	) -> Self {
		Self::Criteria {
			id,
			filter: filter.into_filter(),
			capture: Some(capture.as_ref().to_string()),
		}
	}
}

impl<F: Filter> PredicatePattern<F> {
	/// Tests if a given predicate matches the pattern.
	///
	/// It will match on the predicate id and the data filter
	pub fn matches(&self, predicate: &Predicate) -> bool {
		match self {
			Self::Any => true,
			Self::Criteria { id, filter, .. } => {
				*id == predicate.id && filter.matches(&predicate.params)
			}
		}
	}

	/// If the predicate matches the pattern, and the pattern has a capture name,
	/// this function will return the capture name.
	pub fn capture(&self, predicate: &Predicate) -> Option<&str> {
		if let Self::Criteria {
			capture: Some(name),
			..
		} = self
		{
			if self.matches(predicate) {
				return Some(name);
			}
		}

		None
	}
}
