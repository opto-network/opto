use {
	super::{
		policies::IntoPoliciesPattern,
		unlock::IntoUnlockPattern,
		DataPattern,
		Filter,
		Hot,
		IntoDataPattern,
		PoliciesPattern,
		UnlockPattern,
	},
	crate::{
		codec::{Decode, Encode},
		Hashable,
		Object,
		Predicate,
	},
	alloc::{
		string::{String, ToString},
		vec::Vec,
	},
};

/// A single named capture inside an object.
///
/// When adding patterns, they can be optionally named by using the `capture_*`
/// methods, in that case whenever a pattern matches, a reference to the item
/// (predicate, data, etc) that matched the pattern will be stored in the
/// `Capture` object.
#[derive(Clone, Hash, PartialEq)]
pub enum Capture<'a> {
	Policy(&'a Object, &'a Predicate, usize),
	Unlock(&'a Object, &'a Predicate, usize),
	Data(&'a Object),
}

impl core::fmt::Debug for Capture<'_> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Policy(object, predicate, index) => f
				.debug_tuple("Policy")
				.field(&object.digest())
				.field(predicate)
				.field(index)
				.finish(),
			Self::Unlock(object, predicate, index) => f
				.debug_tuple("Unlock")
				.field(&object.digest())
				.field(predicate)
				.field(index)
				.finish(),
			Self::Data(object) => f
				.debug_tuple("Data")
				.field(&hex::encode(&object.data))
				.finish(),
		}
	}
}

/// A set of criteria that match objects and their elements
#[derive(Debug, Clone)]
pub struct ObjectPattern<F: Filter = Hot> {
	policies: Option<PoliciesPattern<F>>,
	unlock: Option<UnlockPattern<F>>,
	data: Option<DataPattern<F>>,
	name: Option<String>,
}

impl<F: Filter> Default for ObjectPattern<F> {
	fn default() -> Self {
		Self {
			policies: None,
			unlock: None,
			data: None,
			name: None,
		}
	}
}

impl<F: Filter + PartialEq> PartialEq for ObjectPattern<F> {
	fn eq(&self, other: &Self) -> bool {
		self.policies == other.policies
			&& self.unlock == other.unlock
			&& self.data == other.data
			&& self.name == other.name
	}
}

impl<F: Filter + Encode> Encode for ObjectPattern<F> {
	fn encode(&self) -> Vec<u8> {
		let mut result = Vec::new();
		self.policies.encode_to(&mut result);
		self.unlock.encode_to(&mut result);
		self.data.encode_to(&mut result);
		self.name.encode_to(&mut result);
		result
	}
}

impl<F: Filter + Decode> Decode for ObjectPattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let policies = Option::<PoliciesPattern<F>>::decode(input)?;
		let unlock = Option::<UnlockPattern<F>>::decode(input)?;
		let data = Option::<DataPattern<F>>::decode(input)?;
		let name = Option::<String>::decode(input)?;

		Ok(Self {
			name,
			policies,
			unlock,
			data,
		})
	}
}

impl<F: Filter> ObjectPattern<F> {
	pub fn named(name: impl AsRef<str>) -> Self {
		Self {
			name: Some(name.as_ref().to_string()),
			..Default::default()
		}
	}
}

// composition
impl<F: Filter> ObjectPattern<F> {
	pub fn policies(mut self, pattern: impl IntoPoliciesPattern<F>) -> Self {
		self.policies = Some(pattern.into_policies_pattern());
		self
	}

	pub fn unlock(mut self, pattern: impl IntoUnlockPattern<F>) -> Self {
		self.unlock = Some(pattern.into_unlock_pattern());
		self
	}

	pub fn data(mut self, pattern: impl IntoDataPattern<F>) -> Self {
		self.data = Some(pattern.into_data_pattern());
		self
	}
}

// matching
impl<F: Filter> ObjectPattern<F> {
	pub fn matches(&self, object: &Object) -> bool {
		if self.policies.is_none() && self.unlock.is_none() && self.data.is_none() {
			return false;
		}

		if let Some(policies) = &self.policies {
			if !policies.matches(&object.policies) {
				return false;
			}
		}

		if let Some(unlock) = &self.unlock {
			if !unlock.matches(&object.unlock) {
				return false;
			}
		}

		if let Some(data) = &self.data {
			if !data.matches(&object.data) {
				return false;
			}
		}

		true
	}

	pub fn captures<'a, 'b>(
		&'a self,
		object: &'b Object,
	) -> Vec<(&'a str, Capture<'b>)> {
		if !self.matches(object) {
			return Vec::new();
		}

		let mut captures = Vec::new();

		if let Some(policies) = &self.policies {
			captures.extend(policies.captures(&object.policies).into_iter().map(
				|(name, capture, index)| {
					(name, Capture::Policy(object, capture, index))
				},
			));
		}

		if let Some(unlock) = &self.unlock {
			captures.extend(unlock.capture(&object.unlock).into_iter().map(
				|(name, capture, index)| {
					(name, Capture::Unlock(object, capture, index))
				},
			));
		}

		if let Some(data) = &self.data {
			captures.extend(
				data
					.capture(&object.data)
					.into_iter()
					.map(|name| (name, Capture::Data(object))),
			);
		}

		captures
	}
}

/// The mode in which the policies pattern will match.
#[derive(Clone, Debug, Default, Encode, Decode, PartialEq)]
enum MatchMode {
	/// Then the object cannot have policies attached that are not
	/// defined in the required or optional set. Otherwise, as long as the
	/// required policies are met, the object can have any other policies
	/// attached. If we expect a pattern to be matched by multiple objects,
	/// then it needs to have as many occurances of the pattern as the number
	/// of objects.
	Exact,

	/// As long as the object has all the required policies then it will match.
	#[default]
	Fuzzy,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct ObjectCapture<'a, 'b> {
	pub name: Option<&'a str>,
	pub object: &'b Object,
	pub object_index: usize,
	pub captures: Vec<(&'a str, Capture<'b>)>,
}

impl ObjectCapture<'_, '_> {
	pub fn get<D: Decode>(&self, name: &str) -> Option<D> {
		self
			.captures
			.iter()
			.find(|(n, _)| n == &name)
			.and_then(|(_, c)| {
				match c {
					Capture::Policy(_, p, _) => D::decode(&mut p.params.as_slice()),
					Capture::Unlock(_, p, _) => D::decode(&mut p.params.as_slice()),
					Capture::Data(o) => D::decode(&mut o.data.as_slice()),
				}
				.ok()
			})
	}
}

#[derive(Clone, Debug)]
pub struct ObjectsSetPattern<F: Filter = Hot> {
	required: Vec<ObjectPattern<F>>,
	optional: Vec<ObjectPattern<F>>,
	mode: MatchMode,
}

impl<F: Filter + PartialEq> PartialEq for ObjectsSetPattern<F> {
	fn eq(&self, other: &Self) -> bool {
		self.required == other.required
			&& self.optional == other.optional
			&& self.mode == other.mode
	}
}

impl<F: Filter + Encode> Encode for ObjectsSetPattern<F> {
	fn encode(&self) -> Vec<u8> {
		let mut result = Vec::new();

		result.extend_from_slice(&self.required.encode());
		result.extend_from_slice(&self.optional.encode());
		result.extend_from_slice(&self.mode.encode());

		result
	}
}

impl<F: Filter + Decode> Decode for ObjectsSetPattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let required = Vec::<ObjectPattern<F>>::decode(input)?;
		let optional = Vec::<ObjectPattern<F>>::decode(input)?;
		let mode = MatchMode::decode(input)?;

		Ok(Self {
			required,
			optional,
			mode,
		})
	}
}

impl<F: Filter> Default for ObjectsSetPattern<F> {
	fn default() -> Self {
		Self {
			required: Vec::new(),
			optional: Vec::new(),
			mode: MatchMode::Fuzzy,
		}
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn exact() -> Self {
		Self {
			mode: MatchMode::Exact,
			..Default::default()
		}
	}

	pub fn fuzzy() -> Self {
		Self::default()
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn must_include(mut self, pattern: ObjectPattern<F>) -> Self {
		self.required.push(pattern);
		self
	}

	pub fn may_include(mut self, pattern: ObjectPattern<F>) -> Self {
		self.optional.push(pattern);
		self
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn matches(&self, objects: &[Object]) -> bool {
		match self.mode {
			MatchMode::Fuzzy => {
				// in fuzzy mode we need to check that all required patterns are matched

				if self.required.is_empty() {
					return false;
				}

				for pattern in &self.required {
					if !objects.iter().any(|object| pattern.matches(object)) {
						return false;
					}
				}

				true
			}
			MatchMode::Exact => {
				// In exact mode only patterns that are defined in the required or
				// optional set are allowed. All required patterns must be matched,
				// optionals may or may not be matched.

				let mut remaining_objects = (0..objects.len()).collect::<Vec<_>>();
				let mut remaining_required =
					(0..self.required.len()).collect::<Vec<_>>();
				let mut remaining_optional =
					(0..self.optional.len()).collect::<Vec<_>>();

				while let Some(index) = remaining_objects.pop() {
					let object = &objects[index];

					if let Some(matching) = remaining_required
						.iter()
						.position(|&i| self.required[i].matches(object))
					{
						remaining_required.remove(matching);
						continue;
					}

					if let Some(matching) = remaining_optional
						.iter()
						.position(|&i| self.optional[i].matches(object))
					{
						remaining_optional.remove(matching);
						continue;
					}

					// an object did not match any of the required or optional patterns
					return false;
				}

				remaining_required.is_empty()
			}
		}
	}

	/// Captures matching objects and their internal elements.
	///
	/// This method will return captures for patterns that:
	///  - are named object patterns and match the object
	///  - are unnamed object patterns but have named captures
	pub fn captures<'a, 'b>(
		&'a self,
		objects: &'b [Object],
	) -> Vec<ObjectCapture<'a, 'b>> {
		if !self.matches(objects) {
			return Vec::new();
		}

		let mut captures = Vec::new();

		for pattern in &self.required {
			for (i, object) in objects.iter().enumerate() {
				if pattern.matches(object) {
					let object_captures = pattern.captures(object);

					if let Some(name) = pattern.name.as_deref() {
						captures.push(ObjectCapture {
							name: Some(name),
							object,
							object_index: i,
							captures: object_captures,
						});
					} else if !object_captures.is_empty() {
						captures.push(ObjectCapture {
							name: None,
							object,
							object_index: i,
							captures: object_captures,
						});
					}
				}
			}
		}

		for pattern in &self.optional {
			for (i, object) in objects.iter().enumerate() {
				if pattern.matches(object) {
					let object_captures = pattern.captures(object);

					if let Some(name) = pattern.name.as_deref() {
						captures.push(ObjectCapture {
							name: Some(name),
							object,
							object_index: i,
							captures: object_captures,
						});
					} else if !object_captures.is_empty() {
						captures.push(ObjectCapture {
							name: None,
							object,
							object_index: i,
							captures: object_captures,
						});
					}
				}
			}
		}

		captures
	}
}

impl<F: Filter> FromIterator<ObjectPattern<F>> for ObjectsSetPattern<F> {
	fn from_iter<T: IntoIterator<Item = ObjectPattern<F>>>(iter: T) -> Self {
		let mut set = Self::default();

		for pattern in iter {
			set = set.must_include(pattern);
		}

		set
	}
}

pub trait ToObjectSetPattern<F: Filter> {
	fn to_object_set(self) -> ObjectsSetPattern<F>;
}

impl<F: Filter> From<ObjectPattern<F>> for ObjectsSetPattern<F> {
	fn from(pattern: ObjectPattern<F>) -> Self {
		Self::default().must_include(pattern)
	}
}

impl<F: Filter> ToObjectSetPattern<F> for ObjectsSetPattern<F> {
	fn to_object_set(self) -> ObjectsSetPattern<F> {
		self
	}
}

impl<F: Filter> ToObjectSetPattern<F> for ObjectPattern<F> {
	fn to_object_set(self) -> ObjectsSetPattern<F> {
		self.into()
	}
}

#[cfg(test)]
mod tests {
	use {
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

		let pattern = ObjectPattern::named("BaycNft")
			.policies(
				PoliciesPattern::exact()
					.require(NFT_POLICY.named("identity").with_params(
						move |nft: NftIdentity| nft.mint == Digest::compute(b"BAYC'2025"),
					))
					.require(PredicatePattern::new(UNIQUE_POLICY))
					.allow(PredicatePattern::new(NONCE_POLICY)),
			)
			.unlock(UnlockPattern::fuzzy(PredicatePattern::named(
				"owner",
				SIGNATURE_POLICY,
			)))
			.data(DataPattern::named("attribs", Anything));

		assert!(pattern.matches(&nft));

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
}
