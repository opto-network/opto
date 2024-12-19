use {
	crate::{Object, Predicate, PredicateId, Transition},
	alloc::{
		borrow::ToOwned,
		collections::BTreeMap,
		rc::Rc,
		string::String,
		vec::Vec,
	},
	scale::Decode,
};

/// A single named capture inside an object.
///
/// When adding patterns, they can be optionally named by using the `capture_*`
/// methods, in that case whenever a pattern matches, a reference to the item
/// (predicate, data, etc) that matched the pattern will be stored in the
/// `Capture` object.
#[derive(Clone)]
pub enum Capture<'a> {
	Policy(&'a Object, &'a Predicate),
	Unlock(&'a Object, &'a Predicate, usize),
	Data(&'a Object),
}

type CaptureFn = dyn Fn(&[u8]) -> bool;

#[derive(Clone)]
struct Criterion {
	capture: Option<String>,
	filter_fn: Rc<CaptureFn>,
}

/// A set of criteria that match objects and their elements
#[derive(Default, Clone)]
pub struct ObjectPattern {
	data_criteria: Vec<Criterion>,
	policy_criteria: BTreeMap<PredicateId, Vec<Criterion>>,
}

// selectors
impl ObjectPattern {
	/// Checks if an object has a given policy attached to it.
	pub fn has_policy(self, policy: PredicateId) -> Self {
		self.match_policy(policy, |_: ()| true)
	}

	/// Checks if an object has a given policy attached to it that meet specified
	/// criteria.
	pub fn match_policy<T: Decode + 'static>(
		self,
		policy: PredicateId,
		op: impl Fn(T) -> bool + 'static,
	) -> Self {
		self.capture_policy(policy, op, Option::<&str>::None)
	}

	/// Captures all policy predicates that meet the specified criteria.
	/// Allows to name the capture.
	pub fn capture_policy<T: Decode + 'static>(
		mut self,
		policy: PredicateId,
		op: impl Fn(T) -> bool + 'static,
		capture: Option<impl AsRef<str>>,
	) -> Self {
		self
			.policy_criteria
			.entry(policy)
			.or_default()
			.push(Criterion {
				capture: capture.map(|s| s.as_ref().to_owned()),
				filter_fn: Rc::new(move |data: &[u8]| {
					let Ok(value) = T::decode(&mut &data[..]) else {
						return false;
					};

					op(value)
				}),
			});
		self
	}

	/// Checks if an object has data equal to a given value.
	pub fn data_equals<T: Decode + PartialEq + 'static>(self, value: T) -> Self {
		self.match_data(move |data: T| value == data)
	}

	/// Checks if an object has data that meets specified criteria.
	pub fn match_data<T: Decode + 'static>(
		self,
		op: impl Fn(T) -> bool + 'static,
	) -> Self {
		self.capture_data(op, Option::<&str>::None)
	}

	/// Captures objects that have data that meet specified criteria.
	pub fn capture_data<T: Decode + 'static>(
		mut self,
		op: impl Fn(T) -> bool + 'static,
		capture: Option<impl AsRef<str>>,
	) -> Self {
		let filter_fn = Rc::new(move |data: &[u8]| {
			let Ok(value) = T::decode(&mut &data[..]) else {
				return false;
			};
			op(value)
		});

		self.data_criteria.push(Criterion {
			capture: capture.map(|s| s.as_ref().to_owned()),
			filter_fn,
		});
		self
	}
}

// matching
impl ObjectPattern {
	/// Checks if any of the specified match or capture criteria are met on a
	/// given object.
	pub fn is_match(&self, object: &Object) -> bool {
		if object.policies.iter().any(|policy| {
			self
				.policy_criteria
				.get(&policy.id)
				.map_or(false, |criteria| {
					criteria
						.iter()
						.any(|criterion| (criterion.filter_fn)(&policy.params))
				})
		}) {
			return true;
		}

		self
			.data_criteria
			.iter()
			.any(|criterion| (criterion.filter_fn)(&object.data))
	}

	/// Returns a list of captured object or predicates that match the specified
	/// patterns.
	///
	/// Patterns that were named will have their name returned in the result,
	/// unnamed captures or `is_match` results will have `None` as their name.
	pub fn matches<'a, 'b>(
		&'a self,
		object: &'b Object,
	) -> Vec<(Option<&'a str>, Capture<'b>)> {
		let mut captures = Vec::new();

		for policy in object.policies.iter() {
			for (id, criteria) in &self.policy_criteria {
				if policy.id != *id {
					continue;
				}

				for criterion in criteria {
					if (criterion.filter_fn)(&policy.params) {
						captures.push((
							criterion.capture.as_deref(),
							Capture::Policy(object, policy),
						));
					}
				}
			}
		}

		for criterion in &self.data_criteria {
			if (criterion.filter_fn)(&object.data) {
				captures.push((criterion.capture.as_deref(), Capture::Data(object)));
			}
		}

		captures
	}
}

/// Matches objects inside a transition.
///
/// Pattern matches are only applicable to output and ephemerally consumed
/// objects. Input objects do not match as they are expressed as digests only in
/// the compact form of a transition.
#[derive(Default)]
pub struct TransitionPattern {
	output_patterns: Vec<ObjectPattern>,
	ephemeral_patterns: Vec<ObjectPattern>,
}

// selectors
impl TransitionPattern {
	/// Adds a pattern that matches objects produced by a transition.
	pub fn output(mut self, pattern: ObjectPattern) -> Self {
		self.output_patterns.push(pattern);
		self
	}

	/// Adds a pattern that matches ephemeral objects consumed by a transition.
	pub fn ephemeral(mut self, pattern: ObjectPattern) -> Self {
		self.ephemeral_patterns.push(pattern);
		self
	}

	/// Adds a pattern that matches any ephemeral or output object in the
	/// transition.
	pub fn any(mut self, pattern: ObjectPattern) -> Self {
		self.output_patterns.push(pattern.clone());
		self.ephemeral_patterns.push(pattern);
		self
	}

	pub fn is_match(&self, transition: &Transition) -> bool {
		transition.outputs.iter().any(|object| {
			self
				.output_patterns
				.iter()
				.any(|pattern| pattern.is_match(object))
		}) || transition.ephemerals.iter().any(|object| {
			self
				.ephemeral_patterns
				.iter()
				.any(|pattern| pattern.is_match(object))
		})
	}

	pub fn matches<'a, 'b>(
		&'a self,
		transition: &'b Transition,
	) -> Vec<(Option<&'a str>, Capture<'b>)> {
		let mut captures = Vec::new();

		for object in &transition.outputs {
			for pattern in &self.output_patterns {
				captures.extend(pattern.matches(object));
			}
		}

		for object in &transition.ephemerals {
			for pattern in &self.ephemeral_patterns {
				captures.extend(pattern.matches(object));
			}
		}

		captures
	}
}
