use {
	super::{private, Filter, Hot, IntoFilter},
	crate::{
		codec::{Decode, Encode},
		Predicate,
		PredicateId,
	},
	alloc::{
		string::{String, ToString},
		vec::Vec,
	},
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
#[derive(Clone)]
pub enum PredicatePattern<F: Filter = Hot> {
	Any,
	Criteria {
		id: PredicateId,
		filter: F,
		capture: Option<String>,
	},
}

impl<F: Filter> core::fmt::Debug for PredicatePattern<F> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Any => write!(f, "Any"),
			Self::Criteria {
				id,
				filter,
				capture,
			} => {
				let name = match capture {
					Some(name) => name,
					None => "<unnamed>",
				};

				write!(f, "({:?}, {:?}, {:?})", id, filter, name)
			}
		}
	}
}

impl<F: Filter + PartialEq> PartialEq for PredicatePattern<F> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Any, Self::Any) => true,
			(
				Self::Criteria {
					id: id1,
					filter: filter1,
					capture: capture1,
				},
				Self::Criteria {
					id: id2,
					filter: filter2,
					capture: capture2,
				},
			) => id1 == id2 && filter1 == filter2 && capture1 == capture2,
			_ => false,
		}
	}
}

impl<F: Filter + Encode> Encode for PredicatePattern<F> {
	fn encode(&self) -> Vec<u8> {
		match self {
			Self::Any => alloc::vec![0],
			Self::Criteria {
				id,
				filter,
				capture,
			} => {
				let mut result = alloc::vec![1];
				result.extend_from_slice(&id.encode());
				result.extend_from_slice(&filter.encode());
				result.extend_from_slice(&capture.encode());

				result
			}
		}
	}
}

impl<F: Filter + Decode> scale::Decode for PredicatePattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let tag = input.read_byte()?;
		match tag {
			0 => Ok(Self::Any),
			1 => {
				let id = PredicateId::decode(input)?;
				let filter = F::decode(input)?;
				let capture = Option::<String>::decode(input)?;

				Ok(Self::Criteria {
					id,
					filter,
					capture,
				})
			}
			_ => Err("Invalid tag".into()),
		}
	}
}

pub trait IntoPredicatePattern<F: Filter> {
	fn into_predicate_pattern(self) -> PredicatePattern<F>;
}

impl<F: Filter> IntoPredicatePattern<F> for PredicatePattern<F> {
	fn into_predicate_pattern(self) -> PredicatePattern<F> {
		self
	}
}

impl<F: Filter> IntoPredicatePattern<F> for PredicateId {
	fn into_predicate_pattern(self) -> PredicatePattern<F> {
		PredicatePattern::new(self)
	}
}

impl<F: Filter> PredicatePattern<F> {
	pub fn name(&self) -> Option<&str> {
		match self {
			Self::Any => None,
			Self::Criteria { capture, .. } => capture.as_deref(),
		}
	}
}

impl<F: Filter> From<PredicateId> for PredicatePattern<F> {
	fn from(id: PredicateId) -> Self {
		Self::Criteria {
			id,
			filter: F::any(),
			capture: None,
		}
	}
}

impl<F: Filter> PredicatePattern<F> {
	/// Wildcard pattern that matches any predicate
	/// cannot be captured.
	pub fn any() -> Self {
		Self::Any
	}

	/// Create new unnamed predicate pattern. This will be used to match a
	/// predicate without capturing it.
	pub fn new(id: PredicateId) -> Self {
		Self::Criteria {
			id,
			filter: F::any(),
			capture: None,
		}
	}

	/// Create new named predicate pattern.
	///
	/// If this pattern matches a predicate, then the matched predicate will be
	/// accessible by the given name in the capture set.
	pub fn named(capture: impl AsRef<str>, id: PredicateId) -> Self {
		Self::Criteria {
			id,
			filter: F::any(),
			capture: Some(capture.as_ref().to_string()),
		}
	}

	/// Adds a filtering to the predicate params.
	pub fn with_params<T>(mut self, filter: impl IntoFilter<F, T>) -> Self {
		if let Self::Criteria {
			filter: ref mut f, ..
		} = self
		{
			*f = filter.into_filter();
		}

		self
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

pub trait PredicateIdExt: private::Sealed {
	fn params<D: Encode>(self, params: D) -> Predicate;
	fn to_predicate(self) -> Predicate;

	fn named<F: Filter>(self, name: impl AsRef<str>) -> PredicatePattern<F>;
	fn with_params<F: Filter, X>(
		self,
		filter: impl IntoFilter<F, X>,
	) -> PredicatePattern<F>;
}

impl PredicateIdExt for PredicateId {
	fn params<D: Encode>(self, params: D) -> Predicate {
		Predicate {
			id: self,
			params: params.encode(),
		}
	}

	fn to_predicate(self) -> Predicate {
		{
			Predicate {
				id: self,
				params: Default::default(),
			}
		}
	}

	fn named<F: Filter>(self, name: impl AsRef<str>) -> PredicatePattern<F> {
		PredicatePattern::named(name, self)
	}

	fn with_params<F: Filter, X>(
		self,
		filter: impl IntoFilter<F, X>,
	) -> PredicatePattern<F> {
		PredicatePattern::Criteria {
			id: self,
			filter: filter.into_filter(),
			capture: None,
		}
	}
}

impl private::Sealed for PredicateId {}
