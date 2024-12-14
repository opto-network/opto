use opto_core::{Hashable, Object, Transition};

pub trait ObjectExt {
	fn spawn(self) -> Transition;
	fn destroy(&self) -> Transition;
}

impl ObjectExt for Object {
	fn spawn(self) -> Transition {
		Transition {
			inputs: Vec::new(),
			ephemerals: Vec::new(),
			outputs: vec![self],
		}
	}

	fn destroy(&self) -> Transition {
		Transition {
			inputs: vec![self.digest()],
			ephemerals: Vec::new(),
			outputs: Vec::new(),
		}
	}
}
