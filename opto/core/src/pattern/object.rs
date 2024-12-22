use {
	super::{
		Capture,
		Cold,
		DataCriterion,
		Filter,
		Hot,
		PoliciesPattern,
		UnlockPattern,
	},
	crate::Object,
};

/// A set of criteria that match objects and their elements
#[derive(Clone)]
pub struct ObjectPattern<F: Filter = Hot> {
	policies: Option<PoliciesPattern<F>>,
	unlock: Option<UnlockPattern<F>>,
	data: Option<DataCriterion<F>>,
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
			data: None,
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

	pub fn data(mut self, pattern: DataCriterion<F>) -> Self {
		self.data = Some(pattern);
		self
	}
}

// matching
impl<F: Filter> ObjectPattern<F> {
	pub fn matches(&self, object: &Object) -> bool {
		match (&self.policies, &self.data) {
			(None, None) => false,
			(Some(policies), None) => policies.matches(object),
			(None, Some(data)) => data.matches(&object.data),
			(Some(policies), Some(data)) => {
				policies.matches(object) && data.matches(&object.data)
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
	required: Vec<ObjectPattern<F>>,
	optional: Vec<ObjectPattern<F>>,
	exact: bool,
}

impl<F: Filter> Default for ObjectsSetPattern<F> {
	fn default() -> Self {
		Self {
			required: Vec::new(),
			optional: Vec::new(),
			exact: false,
		}
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn must_include(&self, pattern: ObjectPattern) -> Self {
		todo!()
	}

	pub fn may_include(&self, pattern: ObjectPattern) -> Self {
		todo!()
	}
}

impl<F: Filter> ObjectsSetPattern<F> {
	pub fn matches(&self, object: &Object) -> bool {
		todo!()
	}

	pub fn captures(&self, object: &Object) -> Vec<(Option<&str>, Capture)> {
		todo!()
	}
}
