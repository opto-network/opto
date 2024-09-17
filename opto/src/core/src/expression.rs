use crate::repr::AtRest;

/// Represents the basic operators supported by the expression tree.
///
/// Those operators are stored in prefix (polish notation) format
/// and their list is used to construct the expression tree.
pub enum Op<P> {
	Predicate(P),
	And,
	Or,
	Not,
}

/// An expression tree that represents a boolean expression of predicates used
/// in unlock conditions of an object.
///
/// The expression must be evaluated to true for the object to be consumed in an
/// input of a state transition
///
/// The expression tree is stored in the prefix (polish) notation.
pub struct Expression<P = AtRest>(Vec<Op<P>>);
