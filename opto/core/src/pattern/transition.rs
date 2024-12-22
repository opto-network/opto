use {
	super::{Capture, Filter, Hot, ObjectsSetPattern},
	crate::{Digest, Transition},
	alloc::vec::Vec,
};

/// Matches objects inside a transition.
///
/// Pattern matches are only applicable to output and ephemerally consumed
/// objects. Input objects do not match as they are expressed as digests only in
/// the compact form of a transition.
pub struct TransitionPattern<F: Filter = Hot> {
	input_objects: Option<Vec<Digest>>,
	output_patterns: Option<ObjectsSetPattern<F>>,
	ephemeral_patterns: Option<ObjectsSetPattern<F>>,
	exact: bool,
}

impl<F: Filter> Default for TransitionPattern<F> {
	fn default() -> Self {
		Self {
			input_objects: None,
			output_patterns: None,
			ephemeral_patterns: None,
			exact: false,
		}
	}
}

// selectors
impl<F: Filter> TransitionPattern<F> {
	pub fn input(mut self, objects: Vec<Digest>) -> Self {
		self.input_objects = Some(objects);
		self
	}

	/// Adds a pattern that matches objects produced by a transition.
	pub fn output(mut self, pattern: ObjectsSetPattern<F>) -> Self {
		self.output_patterns = Some(pattern);
		self
	}

	/// Adds a pattern that matches ephemeral objects consumed by a transition.
	pub fn ephemeral(mut self, pattern: ObjectsSetPattern<F>) -> Self {
		self.ephemeral_patterns = Some(pattern);
		self
	}

	pub fn exact(mut self) -> Self {
		self.exact = true;
		self
	}

	pub fn matches(&self, transition: &Transition) -> bool {
		todo!()
	}

	pub fn captures<'a, 'b>(
		&'a self,
		transition: &'b Transition,
	) -> Vec<(Option<&'a str>, Capture<'b>)> {
		todo!()
	}
}
