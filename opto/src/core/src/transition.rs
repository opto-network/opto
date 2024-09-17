use crate::{
	alloc::vec::Vec,
	repr::{AsObject, Expanded},
	Repr,
};

/// Represents a state transition.
///
/// This is the basic unit of execution in the system. It is computed
/// off-chain and validated on chain. For a state transition to be
/// valid all policy and unlock predicates on inputes and ephemerals
/// need to be satisfied and all policies on outputs need to be satisfied.
///
/// Input objects are consumed by the transition and are no longer
/// available after the transition is executed. Ephemeral objects are
/// created by the transition and are only available during the execution
/// of the transition. Output objects are created by the transition and
/// are available after the transition is executed.
pub struct Transition<R: Repr = Expanded> {
	pub inputs: Vec<R::InputObject>,
	pub ephemerals: Vec<AsObject<R>>,
	pub outputs: Vec<AsObject<R>>,
}
