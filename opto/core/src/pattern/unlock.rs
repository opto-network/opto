use {
	super::{predicate::PredicatePattern, Filter},
	crate::{Expression, Op, Predicate},
	alloc::vec::Vec,
	core::ops::Range,
};

#[derive(Debug, Clone)]
pub enum MatchMode {
	/// The unlock expression tree must be isomorphic to the pattern and
	/// all corresponding nodes must match the expression.
	Exact,

	/// The unlock expression contains the pattern as a subtree anywhere inside
	/// it.
	Anywhere,
}

#[derive(Clone, Debug)]
pub struct UnlockPattern<F: Filter> {
	expression: Expression<PredicatePattern<F>>,
	mode: MatchMode,
}

// construction
impl<F: Filter> UnlockPattern<F> {
	/// Creates a new unlock pattern matching the given expression exactly.
	pub fn exact(expression: Expression<PredicatePattern<F>>) -> Self {
		Self {
			expression,
			mode: MatchMode::Exact,
		}
	}

	/// Creates a new unlock pattern matching the given expression anywhere in the
	/// unlock expression tree.
	pub fn anywhere(expression: Expression<PredicatePattern<F>>) -> Self {
		Self {
			expression,
			mode: MatchMode::Anywhere,
		}
	}
}

// matching
impl<F: Filter> UnlockPattern<F> {
	/// Checks if a given expression matches the pattern.
	pub fn matches(&self, expr: &Expression) -> bool {
		// get the pattern and expression in prefix form
		let expression_prefix = expr.as_ops();
		let pattern_prefix = self.expression.as_ops();

		self
			.find_matching_range(expression_prefix, pattern_prefix)
			.is_some()
	}

	/// Checks if the given expression matches the pattern and captures
	/// any named patterns, returning a list of tuples with the name of
	/// the capture, a reference to the predicate that matched the capture
	/// and its index in the expression prefix notation.
	///
	/// (_, pred, index) => pred == expr.as_ops()[index]
	pub fn captures<'a, 'b>(
		&'a self,
		expr: &'b Expression,
	) -> Vec<(&'a str, &'b Predicate, usize)> {
		// get the pattern and expression in prefix form
		let expression_prefix = expr.as_ops();
		let pattern_prefix = self.expression.as_ops();

		let Some(range) =
			self.find_matching_range(expression_prefix, pattern_prefix)
		else {
			return Vec::new();
		};

		let offset = range.start;
		let mut captures = vec![];

		for i in range {
			let pattern = &pattern_prefix[i - offset];
			let predicate = &expression_prefix[i];

			if let (Op::Predicate(ref pattern), Op::Predicate(ref predicate)) =
				(pattern, predicate)
			{
				if let Some(capture) = pattern.capture(predicate) {
					captures.push((capture, predicate, i));
				}
			}
		}

		captures
	}

	/// Finds the range of operations in the expression that matches the pattern.
	/// Both the pattern and expression here are in prefix notation.
	fn find_matching_range(
		&self,
		expression_prefix: &[Op<Predicate>],
		pattern_prefix: &[Op<PredicatePattern<F>>],
	) -> Option<Range<usize>> {
		// expressions shorter than the pattern cannot match
		if pattern_prefix.len() > expression_prefix.len() {
			return None;
		}

		if let MatchMode::Exact = self.mode {
			// In exact mode the expression and the pattern must be isomorphic and
			// each predicate must match the corresponding predicate in the
			// expression.
			if pattern_prefix.len() != expression_prefix.len() {
				return None;
			}
		}

		// find a sequence in the expression prefix notation
		// that full matches the pattern prefix notation

		let mut expr_cursor = 0;
		let mut match_cursor = 0;
		let mut pattern_cursor = 0;

		loop {
			match (
				pattern_prefix.get(pattern_cursor),
				expression_prefix.get(match_cursor),
			) {
				(Some(Op::Predicate(pattern)), Some(Op::Predicate(expression))) => {
					if pattern.matches(expression) {
						// advance pattern curosor
						pattern_cursor += 1;
						// advance expression cursor
						match_cursor += 1;
					} else {
						expr_cursor += 1;
						pattern_cursor = 0;
						match_cursor = expr_cursor;
					}
				}
				(Some(Op::And), Some(Op::And))
				| (Some(Op::Or), Some(Op::Or))
				| (Some(Op::Not), Some(Op::Not)) => {
					pattern_cursor += 1;
					match_cursor += 1;
				}
				_ => {
					expr_cursor += 1;
					pattern_cursor = 0;
					match_cursor = expr_cursor;
				}
			};

			if pattern_cursor == pattern_prefix.len() {
				// all ops of the pattern are matched, the expression matches.
				return Some(expr_cursor..match_cursor);
			}

			let remaining_expr = expression_prefix.len() - expr_cursor;
			let remaining_pattern = pattern_prefix.len() - pattern_cursor;

			if remaining_expr < remaining_pattern {
				// no way we can match the pattern anymore
				return None;
			}

			if expr_cursor >= expression_prefix.len() {
				// end of expression, not mached
				return None;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::{
			pattern::{hot::Anything, predicate::PredicatePattern},
			Predicate,
			PredicateId,
		},
		scale::Encode,
	};

	const SIGNATURE_PRED: PredicateId = PredicateId(1);
	const TIME_AFTER_PRED: PredicateId = PredicateId(2);
	const PREIMAGE_PRED: PredicateId = PredicateId(3);

	#[test]
	fn match_single_predicate_exact() {
		let pattern = UnlockPattern::exact(
			PredicatePattern::new(SIGNATURE_PRED, |data: &[u8]| {
				data.starts_with(b"hello")
			})
			.into(),
		);

		// positive
		assert!(pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"hello".to_vec(),
			}
			.into()
		));

		assert!(pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"hello there".to_vec(),
			}
			.into()
		));

		// negative
		assert!(!pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"goodbye".to_vec(),
			}
			.into()
		));

		assert!(!pattern.matches(
			&Predicate {
				id: PREIMAGE_PRED,
				params: b"hello".to_vec(),
			}
			.into()
		));

		let pred1: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: b"goodbye".to_vec(),
		}
		.into();

		let pred2: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"hello".to_vec(),
		}
		.into();

		let expr = pred1 & pred2;
		assert!(!pattern.matches(&expr));
	}

	#[test]
	fn match_single_predicate_exact_with_capture() {
		let pattern = UnlockPattern::exact(
			PredicatePattern::named(
				SIGNATURE_PRED,
				|data: &[u8]| data.starts_with(b"hello"),
				"signature",
			)
			.into(),
		);

		// positive
		assert_eq!(
			pattern.captures(
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello".to_vec(),
				}
				.into()
			),
			vec![(
				"signature",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello".to_vec(),
				},
				0
			)]
		);

		assert_eq!(
			pattern.captures(
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello there".to_vec(),
				}
				.into()
			),
			vec![(
				"signature",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello there".to_vec(),
				},
				0
			)]
		);

		// negative
		assert!(!pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"goodbye".to_vec(),
			}
			.into()
		));

		assert!(!pattern.matches(
			&Predicate {
				id: PREIMAGE_PRED,
				params: b"hello".to_vec(),
			}
			.into()
		));

		let pred1: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: b"goodbye".to_vec(),
		}
		.into();

		let pred2: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"hello".to_vec(),
		}
		.into();

		let expr = pred1 & pred2;
		assert!(!pattern.matches(&expr));
	}

	#[test]
	fn match_exact_non_isomorphic() {
		let signature: Expression<_> = PredicatePattern::named(
			SIGNATURE_PRED,
			|data: &[u8]| data.starts_with(b"hello"),
			"public key",
		)
		.into();

		let time_lock: Expression<_> = PredicatePattern::named(
			TIME_AFTER_PRED,
			|time: u32| time > 15000,
			"time_after",
		)
		.into();

		let expr_pattern: Expression<_> = signature & time_lock;
		let pattern = UnlockPattern::exact(expr_pattern);

		// positive
		let pred1: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"hello".encode(),
		}
		.into();

		let pred2: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: 20000u32.encode(),
		}
		.into();

		let pred3: Expression = Predicate {
			id: PREIMAGE_PRED,
			params: b"hash".to_vec(),
		}
		.into();

		let expr = pred1.clone() & pred2.clone();
		assert!(pattern.matches(&expr));

		assert_eq!(pattern.captures(&expr), vec![
			(
				"public key",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello".encode(),
				},
				1
			),
			(
				"time_after",
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 20000u32.encode(),
				},
				2
			),
		]);

		// negative
		let expr = (pred1 & pred2) | pred3;
		assert!(!pattern.matches(&expr));
	}

	#[test]
	fn match_and_pattern_exact() {
		let signature: Expression<_> = PredicatePattern::named(
			SIGNATURE_PRED,
			|data: &[u8]| data.starts_with(b"hello"),
			"public key",
		)
		.into();

		let time_lock: Expression<_> = PredicatePattern::named(
			TIME_AFTER_PRED,
			|time: u32| time > 15000,
			"time_after",
		)
		.into();

		let expr_pattern: Expression<_> = signature & time_lock;
		let pattern = UnlockPattern::exact(expr_pattern);

		// positive
		let pred1: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"hello".encode(),
		}
		.into();

		let pred2: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: 20000u32.encode(),
		}
		.into();

		let expr = pred1.clone() & pred2.clone();
		assert!(pattern.matches(&expr));

		assert_eq!(pattern.captures(&expr), vec![
			(
				"public key",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"hello".encode(),
				},
				1
			),
			(
				"time_after",
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 20000u32.encode(),
				},
				2
			),
		]);

		// negative
		let unfulfilled_time: Expression<_> = Predicate {
			id: TIME_AFTER_PRED,
			params: 10000u32.encode().to_vec(),
		}
		.into();

		let expr = pred1 & unfulfilled_time;
		assert!(!pattern.matches(&expr));
		assert!(pattern.captures(&expr).is_empty());

		let invalid_signature: Expression<_> = Predicate {
			id: SIGNATURE_PRED,
			params: b"goodbye".encode(),
		}
		.into();

		let expr = invalid_signature & pred2;
		assert!(!pattern.matches(&expr));
		assert!(pattern.captures(&expr).is_empty());
	}

	#[test]
	fn match_sig_with_time_lock_exact() {
		// this expression means:
		// can be unlocked by signature of pub1 after time 15000 or signature of
		// pub2 otherwise

		let signature1: Expression<_> = PredicatePattern::named(
			SIGNATURE_PRED,
			|data: &[u8]| data.starts_with(b"pub1"),
			"pub1",
		)
		.into();

		let signature2: Expression<_> =
			PredicatePattern::named(SIGNATURE_PRED, Anything, "master key").into();

		let time_lock: Expression<_> = PredicatePattern::named(
			TIME_AFTER_PRED,
			|time: u32| time > 15000,
			"time_after",
		)
		.into();

		let expr_pattern: Expression<_> = (signature1 & time_lock) | signature2;
		let pattern = UnlockPattern::exact(expr_pattern);

		// positive
		// pub1 after time 15000
		let pred1: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub1".encode(),
		}
		.into();

		let pred2: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: 20000u32.encode(),
		}
		.into();

		let pred3: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub2".encode(),
		}
		.into();

		let expr = (pred1.clone() & pred2.clone()) | pred3.clone();
		assert!(pattern.matches(&expr));

		assert_eq!(pattern.captures(&expr), vec![
			(
				"pub1",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"pub1".encode(),
				},
				2
			),
			(
				"time_after",
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 20000u32.encode(),
				},
				3
			),
			(
				"master key",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"pub2".encode(),
				},
				4
			)
		]);

		// negative

		// pub1 before time 15000
		let pred2_earlier: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: 10000u32.encode(),
		}
		.into();

		let expr = (pred1.clone() & pred2_earlier) | pred3.clone();
		assert!(!pattern.matches(&expr));

		// pub1 after time 15000 but with invalid signature
		let invalid_signature: Expression<_> = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub3".encode(),
		}
		.into();

		let expr = (invalid_signature & pred2.clone()) | pred3.clone();
		assert!(!pattern.matches(&expr));

		// different tree structure
		let expr = pred1.clone() & (pred2.clone() | pred3.clone());
		assert!(!pattern.matches(&expr));
	}

	#[test]
	fn match_as_long_as_sig_unlocks() {
		let sig =
			PredicatePattern::new(SIGNATURE_PRED, |pubkey: &[u8]| pubkey == b"pub1");

		let sig: Expression<_> = sig.into();
		let anything: Expression<_> = PredicatePattern::any().into();
		let unlock = UnlockPattern::exact(sig | anything);

		let pred1: Expression<_> = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub1".to_vec(),
		}
		.into();

		let pred2: Expression<_> = Predicate {
			id: PREIMAGE_PRED,
			params: b"somehash".to_vec(),
		}
		.into();

		// positive
		let expr = pred1.clone() | pred2.clone();
		assert!(unlock.matches(&expr));

		// negative
		let expr = pred1 & pred2;
		assert!(!unlock.matches(&expr));
	}

	#[test]
	fn match_anywhere_single_pred() {
		let pattern = PredicatePattern::named(
			SIGNATURE_PRED,
			|pubkey: &[u8]| pubkey == b"pub1",
			"mysig",
		);

		let unlock_pattern = UnlockPattern::anywhere(pattern.into());

		// positive, matches a tree with exact structure
		assert!(unlock_pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"pub1".to_vec(),
			}
			.into()
		));

		// negative, a tree with exact structure, non-matching pred
		assert!(!unlock_pattern.matches(
			&Predicate {
				id: SIGNATURE_PRED,
				params: b"pub2".to_vec(),
			}
			.into()
		));

		let sig_pred: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub1".to_vec(),
		}
		.into();

		let time_lock: Expression = Predicate {
			id: TIME_AFTER_PRED,
			params: 800000u64.encode(),
		}
		.into();

		let preimage: Expression = Predicate {
			id: PREIMAGE_PRED,
			params: b"hash1".to_vec(),
		}
		.into();

		let expr: Expression = (sig_pred & time_lock.clone()) | preimage.clone();
		assert!(unlock_pattern.matches(&expr));

		let invalid_sig_pred: Expression = Predicate {
			id: SIGNATURE_PRED,
			params: b"pub2".to_vec(),
		}
		.into();

		let expr: Expression = (invalid_sig_pred & time_lock) | preimage;
		assert!(!unlock_pattern.matches(&expr));
	}

	#[test]
	fn predicates_smoke() {
		assert!(
			PredicatePattern::new(TIME_AFTER_PRED, |time: u64| time < 15000).matches(
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 10000u64.encode(),
				},
			)
		);

		assert!(PredicatePattern::new(SIGNATURE_PRED, |data: &[u8]| data
			== b"pub1")
		.matches(&Predicate {
			id: SIGNATURE_PRED,
			params: b"pub1".encode(),
		}));
	}

	#[test]
	fn match_anywhere_subtree() {
		let signature: Expression<_> = PredicatePattern::named(
			SIGNATURE_PRED,
			|data: &[u8]| data == b"pub1",
			"my sig",
		)
		.into();

		let time_unlocked: Expression<_> = PredicatePattern::named(
			TIME_AFTER_PRED,
			|time: u64| time < 15000u64,
			"vested",
		)
		.into();

		let expr_pattern: Expression<_> = signature & time_unlocked;
		let pattern = UnlockPattern::anywhere(expr_pattern);

		let sig = |val: &[u8]| -> Expression {
			Predicate {
				id: SIGNATURE_PRED,
				params: val.to_vec(),
			}
			.into()
		};

		let time = |val: u64| -> Expression {
			Predicate {
				id: TIME_AFTER_PRED,
				params: val.encode(),
			}
			.into()
		};

		let preimage = |val: &[u8]| -> Expression {
			Predicate {
				id: PREIMAGE_PRED,
				params: val.encode(),
			}
			.into()
		};

		// positive
		let expr = (sig(b"pub1") & time(10000)) | preimage(b"hash1");
		assert!(pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub1") & time(10000)));
		assert!(pattern.matches(&expr));

		// negative

		// expr shorter than pattern
		let expr = sig(b"pub1");
		assert!(!pattern.matches(&expr));

		// negative
		let expr = (sig(b"pub2") & time(10000)) | preimage(b"hash1");
		assert!(!pattern.matches(&expr));

		let expr = (sig(b"pub1") & time(50000)) | preimage(b"hash1");
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| (sig(b"pub1") & time(50000));
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| (sig(b"pub2") & time(10000));
		assert!(!pattern.matches(&expr));

		let expr =
			((preimage(b"hash1") | preimage(b"hash2")) & time(20000)) | time(10000);
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub1") & time(50000)));
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub2") & time(10000)));
		assert!(!pattern.matches(&expr));
	}

	#[test]
	fn capture_anywhere_subtree() {
		let signature: Expression<_> = PredicatePattern::named(
			SIGNATURE_PRED,
			|data: &[u8]| data == b"pub1",
			"my sig",
		)
		.into();

		let time_unlocked: Expression<_> = PredicatePattern::named(
			TIME_AFTER_PRED,
			|time: u64| time < 15000u64,
			"vested",
		)
		.into();

		let expr_pattern: Expression<_> = signature & time_unlocked;
		let pattern = UnlockPattern::anywhere(expr_pattern);

		let sig = |val: &[u8]| -> Expression {
			Predicate {
				id: SIGNATURE_PRED,
				params: val.to_vec(),
			}
			.into()
		};

		let time = |val: u64| -> Expression {
			Predicate {
				id: TIME_AFTER_PRED,
				params: val.encode(),
			}
			.into()
		};

		let preimage = |val: &[u8]| -> Expression {
			Predicate {
				id: PREIMAGE_PRED,
				params: val.encode(),
			}
			.into()
		};

		// positive
		let expr = (sig(b"pub1") & time(10000)) | preimage(b"hash1");
		assert_eq!(pattern.captures(&expr), vec![
			(
				"my sig",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"pub1".to_vec(),
				},
				2
			),
			(
				"vested",
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 10000u64.encode(),
				},
				3
			)
		]);

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub1") & time(10000)));
		assert!(pattern.matches(&expr));
		assert_eq!(pattern.captures(&expr), vec![
			(
				"my sig",
				&Predicate {
					id: SIGNATURE_PRED,
					params: b"pub1".to_vec(),
				},
				11
			),
			(
				"vested",
				&Predicate {
					id: TIME_AFTER_PRED,
					params: 10000u64.encode(),
				},
				12
			)
		]);

		// negative

		// expr shorter than pattern
		let expr = sig(b"pub1");
		assert!(!pattern.matches(&expr));

		// negative
		let expr = (sig(b"pub2") & time(10000)) | preimage(b"hash1");
		assert!(!pattern.matches(&expr));

		let expr = (sig(b"pub1") & time(50000)) | preimage(b"hash1");
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| (sig(b"pub1") & time(50000));
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| (sig(b"pub2") & time(10000));
		assert!(!pattern.matches(&expr));

		let expr =
			((preimage(b"hash1") | preimage(b"hash2")) & time(20000)) | time(10000);
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub1") & time(50000)));
		assert!(!pattern.matches(&expr));

		let expr = ((preimage(b"hash1") | preimage(b"hash2")) & time(20000))
			| ((time(50000) & preimage(b"hash2")) | (sig(b"pub2") & time(10000)));
		assert!(!pattern.matches(&expr));
	}
}
