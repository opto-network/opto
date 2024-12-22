use {
	super::{Capture, Filter},
	crate::{Expression, Object, PredicateId},
};

#[derive(Debug, Clone)]
pub enum MatchMode {
	/// The unlock expression tree must be isomorphic to the pattern and
	/// all nodes must match the expression.
	Exact,

	/// The unlock expression matched by the pattern is suffient to unlock an
	/// object, but it may contain additional nodes that are not present in the
	/// pattern.
	Sufficient,

	/// The unlock expression contains the pattern as a subtree anywhere inside
	/// it.
	Anywhere,
}

#[derive(Clone)]
pub struct UnlockPattern<F: Filter> {
	expression: Expression<(PredicateId, F)>,
	mode: MatchMode,
}

impl<F: Filter> UnlockPattern<F> {
	pub fn matches(&self, object: &Object) -> bool {
		todo!()
	}

	pub fn captures(&self, object: &Object) -> Vec<(Option<&str>, Capture)> {
		todo!()
	}
}
