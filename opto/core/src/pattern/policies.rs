use {
	super::{predicate::PredicatePattern, Filter, Hot, IntoFilter},
	crate::{Predicate, PredicateId},
	alloc::vec::Vec,
};

/// Encapsulates rules for matching the policies list of a given object.
#[derive(Clone, Debug)]
pub struct PoliciesPattern<F: Filter = Hot> {
	/// All those criteria need to be met for the object to match.
	pub(super) required: Vec<PredicatePattern<F>>,

	/// Those criteria are optional, but if they are met, they will be captured.
	///
	/// If strict mode is enabled, then the object cannot have policies attacheds
	/// that are not defined in the required or optional set.
	pub(super) optional: Vec<PredicatePattern<F>>,

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
		self.required.push(PredicatePattern::new(policy, filter));

		self
	}

	/// In exact mode, the object can include at most one policy that matches the
	/// given filter and id. If not in exact mode, this filter is ignored.
	pub fn may_match<T>(
		mut self,
		policy: PredicateId,
		filter: impl IntoFilter<F, T>,
	) -> Self {
		self.optional.push(PredicatePattern::new(policy, filter));

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
		self
			.required
			.push(PredicatePattern::named(policy, filter, capture));

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
		self
			.optional
			.push(PredicatePattern::named(policy, filter, capture));

		self
	}

	pub fn exact(mut self) -> Self {
		self.exact = true;
		self
	}
}

impl<F: Filter> PoliciesPattern<F> {
	/// Checks if a given object's policies list matches the pattern.
	pub fn matches(&self, policies: &[Predicate]) -> bool {
		if !self.exact {
			// if we're not matching in exact mode, then just check if all required
			// policies patterns are satisfied.
			self.required.iter().all(|pattern| {
				policies.iter().any(|predicate| pattern.matches(predicate))
			})
		} else {
			// if we're matching in exact mode then each required pattern must
			// correspond to exactly one policy. Once all required patterns are
			// matched and there are still policies left, then each of them must
			// match exactly one optional pattern. otherwise, the object does not
			// match.

			let mut remaining_policies = (0..policies.len()).collect::<Vec<_>>();
			let mut remaining_required = (0..self.required.len()).collect::<Vec<_>>();
			let mut remaining_optional = (0..self.optional.len()).collect::<Vec<_>>();

			while let Some(policy) = remaining_policies.pop() {
				let policy = &policies[policy];

				// Check if policy matches any of the required patterns
				if let Some(matching) = remaining_required
					.iter()
					.position(|&i| self.required[i].matches(policy))
				{
					// matched one of the required patterns
					remaining_required.remove(matching);
					continue;
				}

				// Check if policy matches any of the optional patterns
				if let Some(matching) = remaining_optional
					.iter()
					.position(|&i| self.optional[i].matches(policy))
				{
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
		policies: &'b [Predicate],
	) -> Vec<(&'a str, &'b Predicate, usize)> {
		if !self.matches(policies) {
			return vec![];
		}

		let mut captures = Vec::new();

		for pattern in &self.required {
			for (i, policy) in policies.iter().enumerate() {
				if let Some(name) = pattern.capture(policy) {
					captures.push((name, policy, i));
				}
			}
		}

		for pattern in &self.optional {
			for (i, policy) in policies.iter().enumerate() {
				if let Some(name) = pattern.capture(policy) {
					captures.push((name, policy, i));
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
		crate::Predicate,
		scale::{Decode, Encode},
	};

	const POLICY_1: PredicateId = PredicateId(1);
	const POLICY_2: PredicateId = PredicateId(2);
	const POLICY_3: PredicateId = PredicateId(3);

	#[test]
	fn require_policy_to_be_present() {
		// use case:
		// - must be an nft
		// - must be a coin

		let pattern = PoliciesPattern::hot().must_include(POLICY_1);

		// positive
		assert!(pattern.matches(&[Predicate {
			id: POLICY_1,
			params: vec![0x01],
		}]));

		// not exact so it may have other policies
		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}
		]));

		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			},
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}
		]));

		// negative
		assert!(!pattern.matches(&[Predicate {
			id: POLICY_2,
			params: vec![0x01],
		}]));
	}

	#[test]
	fn require_exact_policies() {
		let pattern = PoliciesPattern::hot().must_include(POLICY_1).exact();

		// positive
		assert!(pattern.matches(&[Predicate {
			id: POLICY_1,
			params: vec![0x01],
		}]));

		// negative
		assert!(!pattern.matches(&[Predicate {
			id: POLICY_2,
			params: vec![0x01],
		}]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}
		]));
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
		assert!(pattern.matches(&[Predicate {
			id: POLICY_1,
			params: vec![0x01],
		}]));

		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}
		]));

		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			},
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
		]));

		// negative
		assert!(!pattern.matches(&[Predicate {
			id: POLICY_2,
			params: vec![0x01],
		}]));

		// not exact
		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_1,
				params: vec![0x02],
			}
		]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_3,
				params: vec![0x02],
			}
		]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_2,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_3,
				params: vec![0x02],
			}
		]));
	}

	#[test]
	fn require_exact_coin_policy_with_nonce() {
		let pattern = PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_include(POLICY_2)
			.exact();

		// positive
		assert!(pattern.matches(&[Predicate {
			id: POLICY_1,
			params: b"USDC".to_vec(),
		}]));

		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}
		]));

		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			},
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			}
		]));

		// negative
		assert!(!pattern.matches(&[Predicate {
			id: POLICY_1,
			params: b"BTC".to_vec(),
		}]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: b"BTC".to_vec(),
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			}
		]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			},
			Predicate {
				id: POLICY_1,
				params: b"BTC".to_vec(),
			}
		]));

		assert!(!pattern.matches(&[Predicate {
			id: POLICY_2,
			params: vec![0x02],
		}]));

		assert!(!pattern.matches(&[
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
		]));

		assert!(!pattern.matches(&[
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
		]));
	}

	#[test]
	fn require_exact_pattern_with_duplicate_policies() {
		let pattern = PoliciesPattern::hot()
			.must_include(POLICY_1)
			.must_include(POLICY_1)
			.may_include(POLICY_2)
			.exact();

		// positive
		assert!(pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			},
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			},
		]));

		assert!(pattern.matches(&[
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
		]));

		assert!(pattern.matches(&[
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
		]));

		// negative
		assert!(!pattern.matches(&[
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
		]));

		assert!(!pattern.matches(&[
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			},
			Predicate {
				id: POLICY_2,
				params: vec![0x02],
			},
		]));
	}

	#[test]
	fn required_exact_with_type_casting() {
		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params == *b"\x01")
			.may_include(POLICY_2)
			.exact()
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}]));

		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: u32| params == 42)
			.may_include(POLICY_2)
			.exact()
			.matches(&[Predicate {
				id: POLICY_1,
				params: 42.encode(),
			}]));

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
			.matches(&[Predicate {
				id: POLICY_1,
				params: MyStruct { a: 42, b: 43 }.encode(),
			}]));

		assert!(PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params.is_empty())
			.may_include(POLICY_2)
			.exact()
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![],
			}]));

		assert!(!PoliciesPattern::hot()
			.must_match(POLICY_1, |params: &[u8]| params.is_empty())
			.may_include(POLICY_2)
			.exact()
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![0],
			}]));
	}

	#[test]
	fn capture_coin_names() {
		let pattern = PoliciesPattern::hot()
			.must_capture("coin", POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_capture("nonce", POLICY_2, |params: &[u8]| params == *b"42")
			.exact();

		let policies = vec![
			Predicate {
				id: POLICY_1,
				params: b"USDC".to_vec(),
			},
			Predicate {
				id: POLICY_2,
				params: b"42".to_vec(),
			},
		];

		let mut captures = pattern.captures(&policies);
		captures.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));

		assert_eq!(captures.len(), 2);
		assert_eq!(captures[0], ("coin", &policies[0], 0));
		assert_eq!(captures[1], ("nonce", &policies[1], 1));
	}

	#[test]
	fn no_captures_on_no_match() {
		let pattern = PoliciesPattern::hot()
			.must_capture("coin", POLICY_1, |params: &[u8]| params == *b"USDC")
			.may_capture("nonce", POLICY_2, |params: &[u8]| params == *b"42")
			.exact();

		assert!(pattern
			.captures(&[Predicate {
				id: POLICY_2,
				params: b"42".to_vec(),
			}])
			.is_empty());
	}

	#[test]
	fn capture_multiple_matches() {
		let pattern = PoliciesPattern::hot().must_capture(
			"non-empty",
			POLICY_1,
			|params: &[u8]| !params.is_empty(),
		);

		let policies = vec![
			Predicate {
				id: POLICY_1,
				params: vec![0x01],
			},
			Predicate {
				id: POLICY_1,
				params: vec![],
			},
		];

		let captures = pattern.captures(&policies);

		assert_eq!(captures.len(), 1);

		assert_eq!(captures[0], ("non-empty", &policies[0], 0));

		let policies = vec![
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
		];

		let captures = pattern.captures(&policies);

		assert_eq!(captures.len(), 2);

		assert_eq!(captures[0], ("non-empty", &policies[0], 0));

		assert_eq!(captures[1], ("non-empty", &policies[2], 2));
	}
}
