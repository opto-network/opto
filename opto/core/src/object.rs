use {
	super::Expression,
	crate::predicate::AtRest,
	alloc::vec::Vec,
	core::fmt::{Debug, Formatter},
	scale::{self, Decode, Encode, EncodeLike},
	scale_info::TypeInfo,
};

/// The basic and most fundamental unit of state and behavior in the system.
pub struct Object<P = AtRest, D = Vec<u8>> {
	/// A list of predicates that define the type and the behavior of the object.
	/// All predicates must be satisfied for the object to be conlocationred
	/// valid.
	pub policies: Vec<P>,

	/// A boolean expression tree of predicates that must be satisfied for the
	/// object to be a valid input to a state transition.
	pub unlock: Expression<P>,

	/// Arbitrary data associated with the object.
	///
	/// The semantics of this data are defined by the object's policies.
	pub data: D,
}

impl<P: Clone, D: Clone> Clone for Object<P, D> {
	fn clone(&self) -> Self {
		Object {
			policies: self.policies.clone(),
			unlock: self.unlock.clone(),
			data: self.data.clone(),
		}
	}
}

impl<P: Debug, D: Debug> Debug for Object<P, D> {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Object")
			.field("policies", &self.policies)
			.field("unlock", &self.unlock)
			.field("data", &self.data)
			.finish()
	}
}

impl<P: PartialEq, D: PartialEq> PartialEq for Object<P, D> {
	fn eq(&self, other: &Self) -> bool {
		self.policies == other.policies
			&& self.unlock == other.unlock
			&& self.data == other.data
	}
}

/// Serialization support for predicates in states that can be persisted.
impl<P: Encode, D: Encode> Encode for Object<P, D> {
	fn size_hint(&self) -> usize {
		self.data.size_hint()
			+ self.policies.size_hint()
			+ self.unlock.size_hint()
			+ 1 // transient
	}

	fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
		self.policies.encode_to(dest);
		self.unlock.encode_to(dest);
		self.data.encode_to(dest);
	}
}

impl<P: Encode, D: Encode> EncodeLike for Object<P, D> where Object<P, D>: Encode
{}

impl<P: Decode, D: Decode> scale::Decode for Object<P, D> {
	fn decode<I: scale::Input>(value: &mut I) -> Result<Self, scale::Error> {
		Ok(Object {
			policies: Vec::decode(value)?,
			unlock: Expression::decode(value)?,
			data: D::decode(value)?,
		})
	}
}

impl<P: TypeInfo + 'static, D: TypeInfo + 'static> TypeInfo for Object<P, D> {
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("Object", module_path!()))
			.type_params(<[_]>::into_vec(alloc::boxed::Box::new([
				scale_info::TypeParameter::new("P", Some(scale_info::meta_type::<P>())),
				scale_info::TypeParameter::new("D", Some(scale_info::meta_type::<D>())),
			])))
			.composite(
				scale_info::build::Fields::named()
					.field(|f| f.ty::<Vec<P>>().name("policies"))
					.field(|f| f.ty::<Expression<P>>().name("unlock"))
					.field(|f| f.ty::<D>().name("data")),
			)
	}
}

#[cfg(test)]
pub mod tests {
	use {
		super::{super::predicate::PredicateId, *},
		alloc::vec,
		scale::{Decode, Encode},
	};

	pub fn test_object(shift: u32) -> Object<AtRest, Vec<u8>> {
		let pred2: Expression<_> = AtRest {
			id: PredicateId(2 + shift),
			params: vec![3 + shift as u8, 4 + shift as u8, 5 + shift as u8],
		}
		.into();

		let pred3: Expression<_> = AtRest {
			id: PredicateId(3 + shift),
			params: vec![6 + shift as u8, 7 + shift as u8, 8 + shift as u8],
		}
		.into();

		let pred4: Expression<_> = AtRest {
			id: PredicateId(4 + shift),
			params: vec![9 + shift as u8, 11 + shift as u8, 12 + shift as u8],
		}
		.into();

		Object {
			policies: vec![AtRest {
				id: PredicateId(shift),
				params: vec![1 + shift as u8, 2 + shift as u8, 3 + shift as u8],
			}],
			unlock: pred2 & (pred3 | pred4),
			data: vec![13 + shift as u8, 14 + shift as u8, 15 + shift as u8],
		}
	}

	#[test]
	fn encode_decode_smoke() {
		let object = test_object(0);
		let size_hint = object.size_hint();
		let mut encoded = Vec::with_capacity(object.size_hint());
		object.encode_to(&mut encoded);
		assert!(encoded.len() <= size_hint);

		let decoded =
			Object::<AtRest, Vec<u8>>::decode(&mut encoded.as_slice()).unwrap();
		assert_eq!(object, decoded);
	}
}
