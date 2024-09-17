use crate::{expression::Expression, predicate::Predicate, repr::AtRest};

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
