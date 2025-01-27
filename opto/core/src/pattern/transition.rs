use {
	super::{
		object::{ObjectCapture, ToObjectSetPattern},
		Filter,
		Hot,
		ObjectsSetPattern,
	},
	crate::{
		codec::{Decode, Encode},
		Compact,
		Digest,
		Transition,
	},
	alloc::vec::Vec,
};

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum TransitionCapture<'a, 'b> {
	Input(Digest),
	Output(ObjectCapture<'a, 'b>),
	Ephemeral(ObjectCapture<'a, 'b>),
}

impl TransitionCapture<'_, '_> {
	pub fn as_input(&self) -> Option<&Digest> {
		match self {
			TransitionCapture::Input(digest) => Some(digest),
			_ => None,
		}
	}

	pub fn as_output(&self) -> Option<&ObjectCapture> {
		match self {
			TransitionCapture::Output(capture) => Some(capture),
			_ => None,
		}
	}

	pub fn as_ephemeral(&self) -> Option<&ObjectCapture> {
		match self {
			TransitionCapture::Ephemeral(capture) => Some(capture),
			_ => None,
		}
	}

	pub fn name(&self) -> Option<&str> {
		match self {
			TransitionCapture::Input(_) => None,
			TransitionCapture::Output(capture) => capture.name,
			TransitionCapture::Ephemeral(capture) => capture.name,
		}
	}
}

/// Matches objects inside a transition.
///
/// Pattern matches are only applicable to output and ephemerally consumed
/// objects. Input objects do not match as they are expressed as digests only in
/// the compact form of a transition.
#[derive(Clone, Debug)]
pub struct TransitionPattern<F: Filter = Hot> {
	inputs: Option<Vec<Digest>>,
	outputs: Option<ObjectsSetPattern<F>>,
	ephemerals: Option<ObjectsSetPattern<F>>,
}

impl<F: Filter> Default for TransitionPattern<F> {
	fn default() -> Self {
		Self {
			inputs: None,
			outputs: None,
			ephemerals: None,
		}
	}
}

impl<F: Filter + PartialEq> PartialEq for TransitionPattern<F> {
	fn eq(&self, other: &Self) -> bool {
		self.inputs == other.inputs
			&& self.outputs == other.outputs
			&& self.ephemerals == other.ephemerals
	}
}

impl<F: Filter + Encode> Encode for TransitionPattern<F> {
	fn encode(&self) -> Vec<u8> {
		let mut result = Vec::new();
		result.extend_from_slice(&self.inputs.encode());
		result.extend_from_slice(&self.outputs.encode());
		result.extend_from_slice(&self.ephemerals.encode());
		result
	}
}

impl<F: Filter + Decode> Decode for TransitionPattern<F> {
	fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
		let inputs = Option::<Vec<Digest>>::decode(input)?;
		let outputs = Option::<ObjectsSetPattern<F>>::decode(input)?;
		let ephemerals = Option::<ObjectsSetPattern<F>>::decode(input)?;
		Ok(Self {
			inputs,
			outputs,
			ephemerals,
		})
	}
}

// selectors
impl<F: Filter> TransitionPattern<F> {
	pub fn input(mut self, objects: Vec<Digest>) -> Self {
		self.inputs = Some(objects);
		self
	}

	/// Adds a pattern that matches objects produced by a transition.
	pub fn output(mut self, pattern: impl ToObjectSetPattern<F>) -> Self {
		self.outputs = Some(pattern.to_object_set());
		self
	}

	/// Adds a pattern that matches ephemeral objects consumed by a transition.
	pub fn ephemeral(mut self, pattern: impl ToObjectSetPattern<F>) -> Self {
		self.ephemerals = Some(pattern.to_object_set());
		self
	}

	pub fn matches(&self, transition: &Transition) -> bool {
		if let Some(inputs) = self.inputs.as_ref() {
			if !inputs
				.iter()
				.all(|d| transition.inputs.iter().any(|i| i == d))
			{
				return false;
			}
		}

		if let Some(outputs) = self.outputs.as_ref() {
			if !outputs.matches(&transition.outputs) {
				return false;
			}
		}

		if let Some(ephemerals) = self.ephemerals.as_ref() {
			if !ephemerals.matches(&transition.ephemerals) {
				return false;
			}
		}

		true
	}

	pub fn capture<'a, 'b>(
		&'a self,
		transition: &'b Transition<Compact>,
	) -> Vec<TransitionCapture<'a, 'b>> {
		if !self.matches(transition) {
			return Vec::new();
		}

		let mut captures = Vec::new();

		if let Some(input) = self.inputs.as_ref() {
			for digest in input {
				if transition.inputs.iter().any(|d| d == digest) {
					captures.push(TransitionCapture::Input(*digest));
				}
			}
		}

		if let Some(outputs) = self.outputs.as_ref() {
			for capture in outputs.captures(&transition.outputs) {
				captures.push(TransitionCapture::Output(capture));
			}
		}

		if let Some(ephemerals) = self.ephemerals.as_ref() {
			for capture in ephemerals.captures(&transition.ephemerals) {
				captures.push(TransitionCapture::Ephemeral(capture));
			}
		}

		captures
	}
}

#[cfg(test)]
mod tests {}
