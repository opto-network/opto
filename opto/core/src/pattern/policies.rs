use {
	super::{Capture, DataCriterion, Filter, Hot, IntoFilter},
	crate::{Object, PredicateId},
};

/// Encapsulates rules for matching the policies list of a given object.
#[derive(Clone)]
pub struct PoliciesPattern<F: Filter = Hot> {
	/// All those criteria need to be met for the object to match.
	pub(super) required: Vec<(PredicateId, DataCriterion<F>)>,

	/// Those criteria are optional, but if they are met, they will be captured.
	///
	/// If strict mode is enabled, then the object cannot have policies attacheds
	/// that are not defined in the required or optional set.
	pub(super) optional: Vec<(PredicateId, DataCriterion<F>)>,

	/// If true, then the object cannot have policies attached that are not
	/// defined in the required or optional set. Otherwise, as long as the
	/// required policies are met, the object can have any other policies
	/// attached.
	pub(super) exact: bool,
}

// filtering
impl<F: Filter> PoliciesPattern<F> {
	/// The object will match only if it includes a policy that matches the given
	/// filter and id. If running in exact mode, then the object must include
	/// exactly one policy that matches the given filter and id.
	pub fn must_match<T>(
		mut self,
		policy: PredicateId,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		let filter = filter.into_filter();
		self.required.push((policy, DataCriterion {
			capture: None,
			filter,
		}));

		self
	}

	/// In exact mode, the object can include at most one policy that matches the
	/// given filter and id. If not in exact mode, this filter is ignored.
	pub fn may_match<T>(
		mut self,
		policy: PredicateId,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		let filter = filter.into_filter();
		self.optional.push((policy, DataCriterion {
			capture: None,
			filter,
		}));

		self
	}

	/// Matches required pattern and captures the data if the policy is present.
	/// The predicate data will be accessible through the
	/// capture name if the pattern matches.
	pub fn must_capture<T>(
		mut self,
		capture: &str,
		policy: PredicateId,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		let filter = filter.into_filter();
		self.required.push((policy, DataCriterion {
			capture: Some(capture.to_string()),
			filter,
		}));

		self
	}

	/// Matches optional pattern and captures the data if the policy is present.
	/// The predicate data will be accessible through the capture name if the
	/// pattern matches.
	pub fn may_capture<T>(
		mut self,
		capture: &str,
		policy: PredicateId,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		let filter = filter.into_filter();
		self.optional.push((policy, DataCriterion {
			capture: Some(capture.to_string()),
			filter,
		}));

		self
	}

	pub fn exact(mut self) -> Self {
		self.exact = true;
		self
	}
}

impl<F: Filter> PoliciesPattern<F> {
	/// Checks if a given object's policies list matches the pattern.
	pub fn matches(&self, object: &Object) -> bool {
		if !self.exact {
			// if we're not matching in exact mode, then just check if all required
			// policies patterns are satisfied.
			self.required.iter().all(|(policy, criteria)| {
				object.policies.iter().any(|predicate| {
					predicate.id == *policy && criteria.matches(&predicate.params)
				})
			})
		} else {
			// if we're matching in exact mode then each required pattern must
			// correspond to exactly one policy. Once all required patterns are
			// matched and there are still policies left, then each of them must
			// match exactly one optional pattern. otherwise, the object does not
			// match.

			let mut remaining_policies =
				(0..object.policies.len()).collect::<Vec<_>>();
			let mut remaining_required = (0..self.required.len()).collect::<Vec<_>>();
			let mut remaining_optional = (0..self.optional.len()).collect::<Vec<_>>();

			while let Some(policy) = remaining_policies.pop() {
				let policy = &object.policies[policy];

				// Check if policy matches any of the required patterns
				if let Some(matching) = remaining_required.iter().position(|&i| {
					self.required[i].0 == policy.id
						&& self.required[i].1.matches(&policy.params)
				}) {
					// matched one of the required patterns
					remaining_required.remove(matching);
					continue;
				}

				// Check if policy matches any of the optional patterns
				if let Some(matching) = remaining_optional.iter().position(|&i| {
					self.optional[i].0 == policy.id
						&& self.optional[i].1.matches(&policy.params)
				}) {
					// matched one of the optional patterns
					remaining_optional.remove(matching);
					continue;
				}

				// a policy did not match any of the required or optional patterns
				return false;
			}

			remaining_required.is_empty()
		}
	}

	/// Checks if a given object's policies match the pattern and captures
	/// matches. Captured matches are bound to their capture name.
	pub fn captures<'a, 'b>(
		&'a self,
		object: &'b Object,
	) -> Vec<(&'a str, Capture<'b>)> {
		if !self.matches(object) {
			return vec![];
		}

		let mut captures = Vec::new();

		for (policy_id, criteria) in &self.required {
			if let Some(capture) = &criteria.capture {
				for (i, policy) in object.policies.iter().enumerate() {
					if policy.id == *policy_id && criteria.matches(&policy.params) {
						captures
							.push((capture.as_str(), Capture::Policy(object, policy, i)));
					}
				}
			}
		}

		for (policy_id, criteria) in &self.optional {
			if let Some(capture) = &criteria.capture {
				for (i, policy) in object.policies.iter().enumerate() {
					if policy.id == *policy_id && criteria.matches(&policy.params) {
						captures
							.push((capture.as_str(), Capture::Policy(object, policy, i)));
					}
				}
			}
		}

		captures
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{Expression, Op, Predicate},
		core::cell::LazyCell,
		scale::{Decode, Encode},
	};

	const POLICY_1: PredicateId = PredicateId(1);
	const POLICY_2: PredicateId = PredicateId(2);
	const POLICY_3: PredicateId = PredicateId(3);

	const UNLOCK_1: LazyCell<Expression> = LazyCell::new(|| {
		Expression(vec![Op::Predicate(Predicate {
			id: POLICY_3,
			params: vec![0x01],
		})])
	});

	#[test]
	fn require_policy_to_be_present() {
		// use case:
		// - must be an nft
		// - must be a coin

		let pattern = PoliciesPattern::hot().must_include(POLICY_1);

		// positive
		assert!(pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// not exact so it may have other policies
		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// negative
		assert!(!pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_2,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));
	}

	#[test]
	fn require_exact_policies() {
		let pattern = PoliciesPattern::hot().must_include(POLICY_1).exact();

		// positive
		assert!(pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// negative
		assert!(!pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_2,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));
	}

	#[test]
	fn require_exact_policies_with_optionals() {
		// use case:
		// - must be a coin with an optional nonce and nothing else
		// - must be an nft with an optional nonce and nothing else

		let pattern = PoliciesPattern::hot()
			.must_include(POLICY_1)
			.may_include(POLICY_2)
			.exact();

		// positive
		assert!(pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// negative
		assert!(!pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_2,
				params: vec![0x01],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// not exact
		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_1,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_3,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_2,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_3,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));
	}

	#[test]
	fn require_exact_coin_policy_with_nonce() {
		let pattern = PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_include(POLICY_2)
			.exact();

		// positive
		assert!(pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// negative
		assert!(!pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_1,
				params: b"BTC".to_vec(),
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"BTC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_1,
					params: b"BTC".to_vec(),
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_1,
					params: b"BTC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				}
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));
	}

	#[test]
	fn require_exact_pattern_with_duplicate_policies() {
		let pattern = PoliciesPattern::hot()
			.must_include(POLICY_1)
			.must_include(POLICY_1)
			.may_include(POLICY_2)
			.exact();

		// positive
		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: b"USDC".to_vec(),
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		// negative
		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_1,
					params: vec![0x02],
				},
				Predicate {
					id: POLICY_3,
					params: b"USDC".to_vec(),
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));

		assert!(!pattern.matches(&Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: vec![0x02],
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		}));
	}

	#[test]
	fn required_exact_with_type_casting() {
		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params == *b"\x01")
			.may_include(POLICY_2)
			.exact()
			.matches(&Object {
				policies: vec![Predicate {
					id: POLICY_1,
					params: vec![0x01],
				}],
				unlock: UNLOCK_1.clone(),
				data: [].to_vec(),
			}));

		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: u32| params == 42)
			.may_include(POLICY_2)
			.exact()
			.matches(&Object {
				policies: vec![Predicate {
					id: POLICY_1,
					params: 42.encode(),
				}],
				unlock: UNLOCK_1.clone(),
				data: [].to_vec(),
			}));

		#[derive(Encode, Decode, PartialEq)]
		struct MyStruct {
			a: u32,
			b: u32,
		}

		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: MyStruct| params
				== MyStruct { a: 42, b: 43 })
			.may_include(POLICY_2)
			.exact()
			.matches(&Object {
				policies: vec![Predicate {
					id: POLICY_1,
					params: MyStruct { a: 42, b: 43 }.encode(),
				}],
				unlock: UNLOCK_1.clone(),
				data: [].to_vec(),
			}));

		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params.is_empty())
			.may_include(POLICY_2)
			.exact()
			.matches(&Object {
				policies: vec![Predicate {
					id: POLICY_1,
					params: vec![],
				}],
				unlock: UNLOCK_1.clone(),
				data: [].to_vec(),
			}));

		assert!(!PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params.is_empty())
			.may_include(POLICY_2)
			.exact()
			.matches(&Object {
				policies: vec![Predicate {
					id: POLICY_1,
					params: vec![0],
				}],
				unlock: UNLOCK_1.clone(),
				data: [].to_vec(),
			}));
	}

	#[test]
	fn capture_coin_names() {
		let pattern = PoliciesPattern::hot()
			.must_capture("coin", POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_capture("nonce", POLICY_2, |params: &[u8]| params == *b"42")
			.exact();

		let object = Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: b"USDC".to_vec(),
				},
				Predicate {
					id: POLICY_2,
					params: b"42".to_vec(),
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		};

		let mut captures = pattern.captures(&object);
		captures.sort_by(|(a, _), (b, _)| a.cmp(b));

		assert_eq!(captures.len(), 2);
		assert_eq!(
			captures[0],
			("coin", Capture::Policy(&object, &object.policies[0], 0))
		);
		assert_eq!(
			captures[1],
			("nonce", Capture::Policy(&object, &object.policies[1], 1))
		);
	}

	#[test]
	fn no_captures_on_no_match() {
		let pattern = PoliciesPattern::hot()
			.must_capture("coin", POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_capture("nonce", POLICY_2, |params: &[u8]| params == *b"42")
			.exact();

		let object = Object {
			policies: vec![Predicate {
				id: POLICY_2,
				params: b"42".to_vec(),
			}],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		};

		assert!(pattern.captures(&object).is_empty());
	}

	#[test]
	fn capture_multiple_matches() {
		let pattern = PoliciesPattern::hot().must_capture(
			"non-empty",
			POLICY_1,
			|params: &[u8]| !params.is_empty(),
		);

		let object = Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_1,
					params: vec![],
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		};

		let captures = pattern.captures(&object);

		assert_eq!(captures.len(), 1);

		assert_eq!(
			captures[0],
			(
				"non-empty",
				Capture::Policy(&object, &object.policies[0], 0)
			)
		);

		let object = Object {
			policies: vec![
				Predicate {
					id: POLICY_1,
					params: vec![0x01],
				},
				Predicate {
					id: POLICY_1,
					params: vec![],
				},
				Predicate {
					id: POLICY_1,
					params: vec![0x02],
				},
			],
			unlock: UNLOCK_1.clone(),
			data: [].to_vec(),
		};

		let captures = pattern.captures(&object);

		assert_eq!(captures.len(), 2);

		assert_eq!(
			captures[0],
			(
				"non-empty",
				Capture::Policy(&object, &object.policies[0], 0)
			)
		);

		assert_eq!(
			captures[1],
			(
				"non-empty",
				Capture::Policy(&object, &object.policies[2], 2)
			)
		);
	}
}
