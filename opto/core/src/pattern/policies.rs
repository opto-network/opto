use {
	super::{
		predicate::{IntoPredicatePattern, PredicatePattern},
		Filter,
		Hot,
	},
	crate::Predicate,
	alloc::vec::Vec,
	scale::{Decode, Encode},
};

/// The mode in which the policies pattern will match.
#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
pub(super) enum MatchMode {
	/// Then the object cannot have policies attached that are not
	/// defined in the required or optional set. Otherwise, as long as the
	/// required policies are met, the object can have any other policies
	/// attached.
	Exact,

	/// As long as the object has all the required policies then it will match.
	#[default]
	Fuzzy,
}

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

	/// The matching mode of the pattern. See `MatchMode` for more details.
	pub(super) mode: MatchMode,
}

impl<F: Filter + PartialEq> PartialEq for PoliciesPattern<F> {
	fn eq(&self, other: &Self) -> bool {
		self.required == other.required
			&& self.optional == other.optional
			&& self.mode == other.mode
	}
}

impl<F: Filter + Encode> Encode for PoliciesPattern<F> {
	fn encode_to<W: scale::Output + ?Sized>(&self, dest: &mut W) {
		self.required.encode_to(dest);
		self.optional.encode_to(dest);
		self.mode.encode_to(dest);
	}
}

impl<F: Filter + Decode> Decode for PoliciesPattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let required = Vec::<PredicatePattern<F>>::decode(input)?;
		let optional = Vec::<PredicatePattern<F>>::decode(input)?;
		let mode = MatchMode::decode(input)?;

		Ok(Self {
			required,
			optional,
			mode,
		})
	}
}

impl<F: Filter> PoliciesPattern<F> {
	pub fn exact() -> Self {
		Self {
			required: Vec::new(),
			optional: Vec::new(),
			mode: MatchMode::Exact,
		}
	}

	pub fn fuzzy() -> Self {
		Self {
			required: Vec::new(),
			optional: Vec::new(),
			mode: MatchMode::Fuzzy,
		}
	}
}

// filtering
impl<F: Filter> PoliciesPattern<F> {
	/// The object will match only if it includes a policy that matches the given
	/// filter and id. If running in exact mode, then the object must include
	/// exactly one policy that matches the given filter and id.
	pub fn require(mut self, pattern: impl IntoPredicatePattern<F>) -> Self {
		self.required.push(pattern.into_predicate_pattern());
		self
	}

	/// In exact mode, the object can include at most one policy that matches the
	/// given filter and id. If not in exact mode, this filter is ignored during
	/// matching but still can capture.
	pub fn allow(mut self, pattern: impl IntoPredicatePattern<F>) -> Self {
		self.optional.push(pattern.into_predicate_pattern());
		self
	}
}

impl<F: Filter> PoliciesPattern<F> {
	/// Checks if a given object's policies list matches the pattern.
	pub fn matches(&self, policies: &[Predicate]) -> bool {
		match self.mode {
			MatchMode::Fuzzy => {
				// if we're not matching in exact mode, then just check if all required
				// policies patterns are satisfied.
				self.required.iter().all(|pattern| {
					policies.iter().any(|predicate| pattern.matches(predicate))
				})
			}
			MatchMode::Exact => {
				// if we're matching in exact mode then each required pattern must
				// correspond to exactly one policy. Once all required patterns are
				// matched and there are still policies left, then each of them must
				// match exactly one optional pattern. otherwise, the object does not
				// match.

				let mut remaining_policies = (0..policies.len()).collect::<Vec<_>>();
				let mut remaining_required =
					(0..self.required.len()).collect::<Vec<_>>();
				let mut remaining_optional =
					(0..self.optional.len()).collect::<Vec<_>>();

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
	}

	/// Checks if a given object's policies match the pattern and captures
	/// matches. Captured matches are bound to their capture name.
	pub fn captures<'a, 'b>(
		&'a self,
		policies: &'b [Predicate],
	) -> Vec<(&'a str, &'b Predicate, usize)> {
		if !self.matches(policies) {
			return alloc::vec![];
		}

		let mut captures = Vec::new();

		for pattern in &self.required {
			if let Some(name) = pattern.name() {
				for (i, policy) in policies.iter().enumerate() {
					if pattern.capture(policy).is_some() {
						captures.push((name, policy, i));
					}
				}
			}
		}

		for pattern in &self.optional {
			if let Some(name) = pattern.name() {
				for (i, policy) in policies.iter().enumerate() {
					if pattern.capture(policy).is_some() {
						captures.push((name, policy, i));
					}
				}
			}
		}

		captures
	}
}

pub trait IntoPoliciesPattern<F: Filter> {
	fn into_policies_pattern(self) -> PoliciesPattern<F>;
}

impl<F: Filter> IntoPoliciesPattern<F> for PoliciesPattern<F> {
	fn into_policies_pattern(self) -> PoliciesPattern<F> {
		self
	}
}

impl<F: Filter> IntoPoliciesPattern<F> for PredicatePattern<F> {
	fn into_policies_pattern(self) -> PoliciesPattern<F> {
		PoliciesPattern::fuzzy().require(self)
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{Predicate, PredicateId},
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

		let pattern = <PoliciesPattern>::fuzzy() //
			.require(PredicatePattern::new(POLICY_1));

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
		let pattern =
			<PoliciesPattern>::exact().require(PredicatePattern::new(POLICY_1));

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

		let pattern = <PoliciesPattern>::exact()
			.require(PredicatePattern::new(POLICY_1))
			.allow(PredicatePattern::new(POLICY_2));

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
		let pattern = PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1)
					.with_params(|params: &[u8]| params == *b"USDC"),
			)
			.allow(PredicatePattern::new(POLICY_2));

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
		let pattern = <PoliciesPattern>::exact()
			.require(PredicatePattern::new(POLICY_1))
			.require(PredicatePattern::new(POLICY_1))
			.allow(PredicatePattern::new(POLICY_2));

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
		assert!(PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1)
					.with_params(|params: &[u8]| params == *b"\x01")
			)
			.allow(PredicatePattern::new(POLICY_2))
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![0x01],
			}]));

		assert!(PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1).with_params(|params: u32| params == 42)
			)
			.allow(PredicatePattern::new(POLICY_2))
			.matches(&[Predicate {
				id: POLICY_1,
				params: 42.encode(),
			}]));

		#[derive(Encode, Decode, PartialEq)]
		struct MyStruct {
			a: u32,
			b: u32,
		}

		assert!(PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1)
					.with_params(|params: MyStruct| params == MyStruct { a: 42, b: 43 })
			)
			.allow(PredicatePattern::new(POLICY_2))
			.matches(&[Predicate {
				id: POLICY_1,
				params: MyStruct { a: 42, b: 43 }.encode(),
			}]));

		assert!(PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1)
					.with_params(|params: &[u8]| params.is_empty())
			)
			.allow(PredicatePattern::new(POLICY_2))
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![],
			}]));

		assert!(!PoliciesPattern::exact()
			.require(
				PredicatePattern::new(POLICY_1)
					.with_params(|params: &[u8]| params.is_empty())
			)
			.allow(PredicatePattern::new(POLICY_2))
			.matches(&[Predicate {
				id: POLICY_1,
				params: vec![0],
			}]));
	}

	#[test]
	fn capture_coin_names() {
		let pattern = PoliciesPattern::exact()
			.require(
				PredicatePattern::named("coin", POLICY_1)
					.with_params(|params: &[u8]| params == *b"USDC"),
			)
			.allow(
				PredicatePattern::named("nonce", POLICY_2)
					.with_params(|params: &[u8]| params == *b"42"),
			);

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
		let pattern = PoliciesPattern::exact()
			.require(
				PredicatePattern::named("coin", POLICY_1)
					.with_params(|params: &[u8]| params == *b"USDC"),
			)
			.allow(
				PredicatePattern::named("nonce", POLICY_2)
					.with_params(|params: &[u8]| params == *b"42"),
			);

		assert!(pattern
			.captures(&[Predicate {
				id: POLICY_2,
				params: b"42".to_vec(),
			}])
			.is_empty());
	}

	#[test]
	fn capture_multiple_matches() {
		let pattern = PoliciesPattern::fuzzy().require(
			PredicatePattern::named("non-empty", POLICY_1)
				.with_params(|params: &[u8]| !params.is_empty()),
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
