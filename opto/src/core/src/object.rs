use {
	super::{predicate::Predicate, Expression},
	crate::repr::AtRest,
	alloc::vec::Vec,
	core::fmt::{Debug, Formatter},
	scale::{self, Decode, Encode, EncodeLike},
	scale_info::TypeInfo,
};

/// The basic and most fundamental unit of state and behavior in the system.
pub struct Object<P: Predicate = AtRest, D = Vec<u8>> {
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

impl<P: Predicate + Clone, D: Clone> Clone for Object<P, D> {
	fn clone(&self) -> Self {
		Object {
			policies: self.policies.clone(),
			unlock: self.unlock.clone(),
			data: self.data.clone(),
		}
	}
}

impl<P: Predicate + Debug, D: Debug> Debug for Object<P, D> {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Object")
			.field("policies", &self.policies)
			.field("unlock", &self.unlock)
			.field("data", &self.data)
			.finish()
	}
}

impl<P: Predicate + PartialEq, D: PartialEq> PartialEq for Object<P, D> {
	fn eq(&self, other: &Self) -> bool {
		self.policies == other.policies
			&& self.unlock == other.unlock
			&& self.data == other.data
	}
}

/// Serialization support for predicates in states that can be persisted.
impl<P: Predicate + Encode, D: Encode> Encode for Object<P, D> {
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

impl<P: Predicate + Encode, D: Encode> EncodeLike for Object<P, D> where
	Object<P, D>: Encode
{
}

impl<P: Predicate + Decode, D: Decode> scale::Decode for Object<P, D> {
	fn decode<I: scale::Input>(value: &mut I) -> Result<Self, scale::Error> {
		Ok(Object {
			policies: Vec::decode(value)?,
			unlock: Expression::decode(value)?,
			data: D::decode(value)?,
		})
	}
}

impl<P: Predicate + TypeInfo + 'static, D: TypeInfo + 'static> TypeInfo
	for Object<P, D>
{
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
