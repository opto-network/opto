use {
	super::{private, Anything, Filter, IntoFilter},
	crate::Expression,
	alloc::{format, string::ToString, vec::Vec},
	core::ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo},
	scale::{Decode, Encode},
};

#[derive(Clone, Encode, Decode, PartialEq)]
pub enum Comparison {
	Equal,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
}

impl core::fmt::Debug for Comparison {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Comparison::Equal => write!(f, "=="),
			Comparison::NotEqual => write!(f, "!="),
			Comparison::LessThan => write!(f, "<"),
			Comparison::LessThanOrEqual => write!(f, "<="),
			Comparison::GreaterThan => write!(f, ">"),
			Comparison::GreaterThanOrEqual => write!(f, ">="),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceRange {
	from: Bound<u32>,
	to: Bound<u32>,
}

impl From<Range<usize>> for SourceRange {
	fn from(range: Range<usize>) -> Self {
		Self {
			from: Bound::Included(range.start as u32),
			to: Bound::Excluded(range.end as u32),
		}
	}
}

impl From<RangeInclusive<usize>> for SourceRange {
	fn from(range: RangeInclusive<usize>) -> Self {
		Self {
			from: Bound::Included(*range.start() as u32),
			to: Bound::Included(*range.end() as u32),
		}
	}
}

impl From<RangeFull> for SourceRange {
	fn from(_: RangeFull) -> Self {
		Self {
			from: Bound::Unbounded,
			to: Bound::Unbounded,
		}
	}
}

impl From<RangeFrom<usize>> for SourceRange {
	fn from(range: RangeFrom<usize>) -> Self {
		Self {
			from: Bound::Included(range.start as u32),
			to: Bound::Unbounded,
		}
	}
}

impl From<RangeTo<usize>> for SourceRange {
	fn from(range: RangeTo<usize>) -> Self {
		Self {
			from: Bound::Unbounded,
			to: Bound::Excluded(range.end as u32),
		}
	}
}

impl Encode for SourceRange {
	fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
		match self.from {
			Bound::Included(from) => {
				dest.push_byte(1);
				dest.write(from.to_le_bytes().as_ref());
			}
			Bound::Excluded(from) => {
				dest.push_byte(2);
				dest.write(from.to_le_bytes().as_ref());
			}
			Bound::Unbounded => {
				dest.push_byte(0);
			}
		}

		match self.to {
			Bound::Included(to) => {
				dest.push_byte(1);
				dest.write(to.to_le_bytes().as_ref());
			}
			Bound::Excluded(to) => {
				dest.push_byte(2);
				dest.write(to.to_le_bytes().as_ref());
			}
			Bound::Unbounded => {
				dest.push_byte(0);
			}
		}
	}
}

impl Decode for SourceRange {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let mut buf = [0u8; 4];
		let from = match input.read_byte()? {
			0 => Bound::Unbounded,
			1 => {
				input.read(&mut buf[..])?;
				Bound::Included(u32::from_le_bytes(buf))
			}
			2 => {
				input.read(&mut buf[..])?;
				Bound::Excluded(u32::from_le_bytes(buf))
			}
			_ => return Err(scale::Error::from("Invalid bound")),
		};

		let to = match input.read_byte()? {
			0 => Bound::Unbounded,
			1 => {
				input.read(&mut buf[..])?;
				Bound::Included(u32::from_le_bytes(buf))
			}
			2 => {
				input.read(&mut buf[..])?;
				Bound::Excluded(u32::from_le_bytes(buf))
			}
			_ => return Err(scale::Error::from("Invalid bound")),
		};

		Ok(Self { from, to })
	}
}

#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Condition {
	pub op: Comparison,
	pub value: Vec<u8>,
	pub source: SourceRange,
}

impl core::fmt::Debug for Condition {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		if self.value.is_empty()
			&& self.op == Comparison::Equal
			&& self.source.from == Bound::Unbounded
			&& self.source.to == Bound::Unbounded
		{
			write!(f, "Any")
		} else {
			let formatted_range = match (&self.source.from, &self.source.to) {
				(Bound::Included(from), Bound::Excluded(to)) => {
					format!("{:?}..{:?}", from, to)
				}
				(Bound::Included(from), Bound::Included(to)) => {
					format!("{:?}..={:?}", from, to)
				}
				(Bound::Excluded(from), Bound::Included(to)) => {
					format!("{:?}..={:?}", from, to)
				}
				(Bound::Excluded(from), Bound::Excluded(to)) => {
					format!("{:?}..{:?}", from, to)
				}
				(Bound::Unbounded, Bound::Excluded(to)) => format!("..{:?}", to),
				(Bound::Unbounded, Bound::Included(to)) => format!("..={:?}", to),
				(Bound::Included(from), Bound::Unbounded) => format!("{:?}..", from),
				(Bound::Excluded(from), Bound::Unbounded) => format!("{:?}..", from),
				(Bound::Unbounded, Bound::Unbounded) => "..".to_string(),
			};

			write!(
				f,
				"([{}] {:?} 0x{}",
				formatted_range,
				self.op,
				hex::encode(&self.value)
			)
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Cold(Expression<Condition>);

impl core::fmt::Debug for Cold {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

impl Filter for Cold {
	fn matches(&self, data: &[u8]) -> bool {
		let evaluated = self.0.as_ref().map(|op| {
			let start = match op.source.from {
				Bound::Included(start) => start as usize,
				Bound::Excluded(start) => start as usize + 1,
				Bound::Unbounded => 0,
			};

			let end = match op.source.to {
				Bound::Included(end) => end as usize + 1,
				Bound::Excluded(end) => end as usize,
				Bound::Unbounded => data.len().min(op.value.len()),
			};

			if start == end && op.value.is_empty() {
				return true;
			}

			if start >= data.len() || end > data.len() {
				return false;
			}

			let value = &data[start..end];
			match op.op {
				Comparison::Equal => value == op.value,
				Comparison::NotEqual => value != op.value,
				Comparison::LessThan => value < op.value.as_slice(),
				Comparison::LessThanOrEqual => value <= op.value.as_slice(),
				Comparison::GreaterThan => value > op.value.as_slice(),
				Comparison::GreaterThanOrEqual => value >= op.value.as_slice(),
			}
		});

		evaluated.reduce()
	}

	fn any() -> Self {
		Cold(Expression::from(Condition {
			op: Comparison::Equal,
			value: alloc::vec![],
			source: (..).into(),
		}))
	}
}

impl IntoFilter<Cold, private::Sentinel<()>> for Cold {
	fn into_filter(self) -> Cold {
		self
	}
}

impl IntoFilter<Cold> for Anything {
	fn into_filter(self) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::Equal,
			value: alloc::vec![],
			source: (..).into(),
		}))
	}
}

impl private::Sealed for Cold {}

pub trait Comparable {
	fn value(&self) -> Vec<u8>;
}

impl<T: Encode> Comparable for T {
	fn value(&self) -> Vec<u8> {
		self.encode()
	}
}

pub trait ColdCaptureExt {
	fn equals(self, value: impl Comparable) -> Cold;
	fn not_equals(self, value: impl Comparable) -> Cold;
	fn less_than(self, value: impl Comparable) -> Cold;
	fn less_than_or_equals(self, value: impl Comparable) -> Cold;
	fn greater_than(self, value: impl Comparable) -> Cold;
	fn greater_than_or_equals(self, value: impl Comparable) -> Cold;
}

impl<T> ColdCaptureExt for T
where
	T: Into<SourceRange>,
{
	fn equals(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::Equal,
			value: value.value(),
			source: self.into(),
		}))
	}

	fn not_equals(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::NotEqual,
			value: value.value(),
			source: self.into(),
		}))
	}

	fn less_than(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::LessThan,
			value: value.value(),
			source: self.into(),
		}))
	}

	fn less_than_or_equals(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::LessThanOrEqual,
			value: value.value(),
			source: self.into(),
		}))
	}

	fn greater_than(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::GreaterThan,
			value: value.value(),
			source: self.into(),
		}))
	}

	fn greater_than_or_equals(self, value: impl Comparable) -> Cold {
		Cold(Expression::from(Condition {
			op: Comparison::GreaterThanOrEqual,
			value: value.value(),
			source: self.into(),
		}))
	}
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		crate::*,
		scale::{Decode, Encode},
	};

	const NFT_POLICY: PredicateId = PredicateId(100001);
	const UNIQUE_POLICY: PredicateId = PredicateId(201);
	const NONCE_POLICY: PredicateId = PredicateId(202);
	const SIGNATURE_POLICY: PredicateId = PredicateId(203);
	const PREIMAGE_POLICY: PredicateId = PredicateId(204);

	#[derive(Debug, Encode, Decode)]
	struct NftIdentity {
		pub mint: Digest,
		pub tag: Digest,
		pub mutable: bool,
	}

	#[test]
	fn nft_example_smoke() {
		let nft = Object {
			policies: vec![
				NFT_POLICY.params(NftIdentity {
					mint: Digest::compute(b"BAYC'2025"),
					tag: Digest::compute(b"monkey1"),
					mutable: false,
				}),
				NONCE_POLICY.params(999383u64.to_le_bytes()),
				UNIQUE_POLICY.params(Digest::compute(b"unique1")),
			],
			unlock: SIGNATURE_POLICY.params(b"pub1").into(),
			data: b"attrib1=val1,attrib2=val2".to_vec(),
		};

		let pattern = ObjectPattern::<Cold>::named("BaycNft")
			.policies(
				PoliciesPattern::<Cold>::exact()
					.require(
						NFT_POLICY
							.named("identity")
							.with_params((0..32).equals(Digest::compute(b"BAYC'2025"))),
					)
					.require(PredicatePattern::new(UNIQUE_POLICY))
					.allow(PredicatePattern::new(NONCE_POLICY)),
			)
			.unlock(UnlockPattern::fuzzy(PredicatePattern::named(
				"owner",
				SIGNATURE_POLICY,
			)))
			.data(DataPattern::named("attribs", Anything));

		assert!(pattern.matches(&nft));

		let serialized_pattern = pattern.encode();

		let decoded_pattern =
			ObjectPattern::<Cold>::decode(&mut &serialized_pattern[..])
				.expect("Decoding should work");

		assert_eq!(pattern, decoded_pattern);
		assert!(decoded_pattern.matches(&nft));

		let captures = pattern.captures(&nft);

		assert_eq!(captures.len(), 3);
		assert_eq!(captures, vec![
			("identity", Capture::Policy(&nft, &nft.policies[0], 0)),
			(
				"owner",
				Capture::Unlock(&nft, &SIGNATURE_POLICY.params(b"pub1"), 0)
			),
			("attribs", Capture::Data(&nft)),
		]);

		let captures = decoded_pattern.captures(&nft);

		assert_eq!(captures.len(), 3);
		assert_eq!(captures, vec![
			("identity", Capture::Policy(&nft, &nft.policies[0], 0)),
			(
				"owner",
				Capture::Unlock(&nft, &SIGNATURE_POLICY.params(b"pub1"), 0)
			),
			("attribs", Capture::Data(&nft)),
		]);

		// negative
		let nft = Object {
			policies: vec![
				NFT_POLICY.params(NftIdentity {
					mint: Digest::compute(b"BAYC'2026"),
					tag: Digest::compute(b"monkey1"),
					mutable: false,
				}),
				NONCE_POLICY.params(999383u64.to_le_bytes()),
				UNIQUE_POLICY.params(Digest::compute(b"unique1")),
			],
			unlock: PREIMAGE_POLICY.params(b"pub1").into(),
			data: b"attrib1=val1,attrib2=val2".to_vec(),
		};

		assert!(!pattern.matches(&nft));
		assert!(pattern.captures(&nft).is_empty());
	}

	#[test]
	fn cold_pattern_matches() {
		let pattern = Cold(
			Condition {
				op: Comparison::Equal,
				value: vec![1, 2, 3],
				source: (0..3).into(),
			}
			.into(),
		);

		assert!(pattern.matches(&[1, 2, 3]));
		assert!(!pattern.matches(&[1, 2, 4]));
	}

	#[test]
	fn encoding() {
		let predicate_pattern = PredicatePattern::<Cold>::new(PredicateId(1));
		let encoded = predicate_pattern.encode();
		let decoded = PredicatePattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");
		assert_eq!(decoded, predicate_pattern);

		let policy_pattern = PoliciesPattern::<Cold>::exact()
			.require(PredicatePattern::new(PredicateId(1)))
			.allow(PredicatePattern::new(PredicateId(2)));

		let encoded = policy_pattern.encode();
		let decoded = PoliciesPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");

		assert_eq!(decoded, policy_pattern);

		let unlock_pattern =
			UnlockPattern::<Cold>::fuzzy(PredicatePattern::new(PredicateId(1)));

		let encoded = unlock_pattern.encode();
		let decoded = UnlockPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");

		assert_eq!(decoded, unlock_pattern);

		let data_pattern = DataPattern::<Cold>::named("attribs", Anything);
		let encoded = data_pattern.encode();
		let decoded = DataPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");
		assert_eq!(decoded, data_pattern);

		let object_pattern = ObjectPattern::<Cold>::named("BaycNft")
			.policies(
				PoliciesPattern::<Cold>::exact()
					.require(
						PredicatePattern::new(PredicateId(1))
							.with_params((0..8).equals(1u64)),
					)
					.allow(PredicatePattern::new(PredicateId(2))),
			)
			.unlock(UnlockPattern::fuzzy(
				PredicatePattern::new(PredicateId(3)).with_params((0..4).equals(1u32)),
			))
			.data(DataPattern::named("attribs", Anything));

		let encoded = object_pattern.encode();
		let decoded = ObjectPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");
		assert_eq!(decoded, object_pattern);

		let object_set_pattern = ObjectsSetPattern::<Cold>::exact().must_include(
			ObjectPattern::named("BaycNft")
				.policies(
					PoliciesPattern::<Cold>::exact()
						.require(PredicatePattern::new(PredicateId(1)))
						.allow(PredicatePattern::new(PredicateId(2))),
				)
				.unlock(UnlockPattern::fuzzy(PredicatePattern::new(PredicateId(3))))
				.data(DataPattern::named("attribs", Anything)),
		);

		let encoded = object_set_pattern.encode();
		let decoded = ObjectsSetPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");
		assert_eq!(decoded, object_set_pattern);

		let transition_pattern = TransitionPattern::<Cold>::default()
			.input(vec![Digest::compute(b"input1")])
			.output(object_set_pattern.clone())
			.ephemeral(object_set_pattern.clone());

		let encoded = transition_pattern.encode();
		let decoded = TransitionPattern::<Cold>::decode(&mut &encoded[..])
			.expect("Decoding should work");
		assert_eq!(decoded, transition_pattern);
	}

	#[test]
	fn unsigned_int_type_conversion() {
		let cold: Cold = Anything.into_filter();
		assert!(cold.matches(b""));
		assert!(cold.matches(b"abc"));

		let cold: Cold = (0..1).equals(187u8);
		assert!(cold.matches(&187u8.to_le_bytes()));

		let cold: Cold = (0..2).equals(187u16);
		assert!(cold.matches(&187u16.to_le_bytes()));

		let cold: Cold = (0..4).not_equals(187u32);
		assert!(cold.matches(&188u32.to_le_bytes()));

		let cold: Cold = (0..4).less_than(187u32);
		assert!(cold.matches(&186u32.to_le_bytes()));

		let cold: Cold = (..4).less_than(187u32);
		assert!(cold.matches(&186u32.to_le_bytes()));

		let cold: Cold = (..).less_than(187u32);
		assert!(cold.matches(&186u32.to_le_bytes()));

		let cold: Cold = (0..4).less_than_or_equals(187u32);
		assert!(cold.matches(&187u32.to_le_bytes()));

		let cold: Cold = (0..4).greater_than(187u32);
		assert!(cold.matches(&188u32.to_le_bytes()));

		let cold: Cold = (0..4).greater_than_or_equals(187u32);
		assert!(cold.matches(&187u32.to_le_bytes()));

		let cold: Cold = (0..8).equals(187u64);
		assert!(cold.matches(&187u64.to_le_bytes()));

		let cold: Cold = (..8).equals(187u64);
		assert!(cold.matches(&187u64.to_le_bytes()));

		let cold: Cold = (0..).equals(187u64);
		assert!(cold.matches(&187u64.to_le_bytes()));

		let cold: Cold = (..).equals(187u64);
		assert!(cold.matches(&187u64.to_le_bytes()));

		let cold: Cold = (0..16).equals(187u128);
		assert!(cold.matches(&187u128.to_le_bytes()));

		// negative
		let cold: Cold = (0..16).equals(187u128);
		assert!(!cold.matches(&187u64.to_le_bytes()));

		let cold: Cold = (0..16).equals(187u128);
		assert!(!cold.matches(&187u32.to_le_bytes()));

		let cold: Cold = (0..16).equals(187u128);
		assert!(!cold.matches(&187u16.to_le_bytes()));

		let cold: Cold = (0..).equals(187u128);
		assert!(!cold.matches(&187u16.to_le_bytes()));

		let cold: Cold = (..16).equals(187u128);
		assert!(!cold.matches(&187u16.to_le_bytes()));

		let cold: Cold = (..).equals(187u128);
		assert!(!cold.matches(&187u16.to_le_bytes()));
	}
}
