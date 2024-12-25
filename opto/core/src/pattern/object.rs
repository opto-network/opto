use {
	super::{Capture, Cold, Filter, Hot, PoliciesPattern, UnlockPattern},
	crate::Object,
	alloc::vec::Vec,
};

/// A set of criteria that match objects and their elements
#[derive(Clone)]
pub struct ObjectPattern<F: Filter = Hot> {
	policies: Option<PoliciesPattern<F>>,
	unlock: Option<UnlockPattern<F>>,
}

impl ObjectPattern<Hot> {
	pub fn hot() -> ObjectPattern<Hot> {
		ObjectPattern::default()
	}

	pub fn cold() -> ObjectPattern<Cold> {
		ObjectPattern::default()
	}
}

impl<F: Filter> Default for ObjectPattern<F> {
	fn default() -> Self {
		Self {
			policies: None,
			unlock: None,
		}
	}
}

// composition
impl<F: Filter> ObjectPattern<F> {
	pub fn policies(mut self, pattern: PoliciesPattern<F>) -> Self {
		self.policies = Some(pattern);
		self
	}

	pub fn unlock(mut self, pattern: UnlockPattern<F>) -> Self {
		self.unlock = Some(pattern);
		self
	}
}

// matching
impl<F: Filter> ObjectPattern<F> {
	pub fn matches(&self, object: &Object) -> bool {
		match (&self.policies, &self.unlock) {
			(None, None) => false,
			(Some(policies), None) => policies.matches(&object.policies),
			(None, Some(unlock)) => unlock.matches(&object.unlock),
			(Some(policies), Some(unlock)) => {
				policies.matches(&object.policies) && unlock.matches(&object.unlock)
			}
		}
	}

	pub fn captures<'a: 'b, 'b>(
		&'a self,
		_: &'b Object,
	) -> Vec<(&'a str, Capture<'b>)> {
		todo!()
	}
}

pub struct ObjectsSetPattern<F: Filter = Hot> {
	_required: Vec<ObjectPattern<F>>,
	_optional: Vec<ObjectPattern<F>>,
	_exact: bool,
}

impl<F: Filter> Default for ObjectsSetPattern<F> {
	fn default() -> Self {
		Self {
			_required: Vec::new(),
			_optional: Vec::new(),
			_exact: false,
		}
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn must_include(&self, _pattern: ObjectPattern) -> Self {
		todo!()
	}

	pub fn may_include(&self, _pattern: ObjectPattern) -> Self {
		todo!()
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn matches(&self, _object: &Object) -> bool {
		todo!()
	}

	pub fn captures(&self, _object: &Object) -> Vec<(Option<&str>, Capture)> {
		todo!()
	}
}
