//! This module contains various helpers and QoL extensions for working with
//! transitions. All those operations can be done directly on transitions, but
//! this module provides a more ergonomic way to do so.
//!
//! The helpers in this module do not cover all possible operations on
//! transitions, here we are prioritizing convention and ergonomics over
//! completeness.

use opto_core::{
	repr::{Compact, Expanded},
	Hashable,
	Transition,
};

/// Utilities that apply to compact transitions.
pub trait CompactTransitionExt
where
	Self: Sized,
{
}

/// Utilities that apply to expanded transitions.
trait ExpandedTransitionExt
where
	Self: Sized,
{
	/// Converts an expanded transition to a compact transition.
	fn compact(self) -> Transition<Compact>;
}

impl ExpandedTransitionExt for Transition<Expanded> {
	fn compact(self) -> Transition<Compact> {
		Transition {
			inputs: self.inputs.into_iter().map(|x| x.digest()).collect(),
			ephemerals: self.ephemerals,
			outputs: self.outputs,
		}
	}
}
