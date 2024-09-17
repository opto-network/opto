use {
	crate::{
		expression::Expression,
		object::Object,
		predicate::{Predicate, PredicateId},
		Digest,
	},
	alloc::vec::Vec,
	scale::{Decode, Encode},
	scale_info::TypeInfo,
};

/// Configures the representation of the state transition.
///
/// This is a trait that allows the same state transition to be represented
/// in different stages of it's lifetime:
/// - in transit: where we want to minimize the amount to transferred bytes and
///   remove redundant information, such as the content of input objects. this
///   representation is serializable.
/// - at rest: where we want to store the transition in a database or on disk,
///   or analyze it in some way. Here we want to have all the information
///   available, including the content of input objects. this representation
///   also should be serializable.
/// - in use: where we want to execute the transition. Here we want to have an
///   instance of the state transition with all predicates instantiated by the
///   machine that is executing the transition and ready to be executed. This
///   representation is not serializable. In this state we want to limit the
///   amount of data copying so we use references to the original data in the
///   "at rest" representation. This representation is not serializable, not
///   clonable, copiable or comparable.
pub trait Repr {
	type Data;
	type InputObject;
	type Predicate: Predicate;
}

pub type AsInput<R> = <R as Repr>::InputObject;
pub type AsPredicate<R> = <R as Repr>::Predicate;
pub type AsObject<R> = Object<<R as Repr>::Predicate, <R as Repr>::Data>;
pub type AsExpression<R> = Expression<<R as Repr>::Predicate>;

/// A description of a predicate.
///
/// Describes a predicate that can be instantiated when executed in context of a
/// machine. Predicates in this form are used for persistance and transport.
/// When a predicate needs to be evaluated it must be instantiated with a
/// machine and transformed into `InUse`.
///
/// Predicates in this form can be persisted, cloned, and transported across
/// machines. They have a universal representation and can be used to describe
/// the behavior of objects and state transitions in a way that is independent
/// of the machine that is executing them.
#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode, TypeInfo)]
pub struct AtRest {
	/// A unique identifier for the predicate.
	///
	/// Predicates are known to the blockchain and are identified by a numerical
	/// identifier that is assigned at deployment time. This identifier uniquely
	/// maps an integer to a WASM code blob that contains the predicate's logic.
	///
	/// The contents of the blob can be always retreived from the blockchain by
	/// querying the CID of the predicate and fetching the corresponding code
	/// blob through IPFS.
	///
	/// Most often the code for the most common predicates will be bundled with
	/// the runtime that is executing the state transition.
	pub id: PredicateId,

	/// An arbitrary set of parameters that are passed to the predicate when it
	/// is evaluated. The format and the meaning of the parameters are defined
	/// by the predicate's logic and should be looked up in the predicate's
	/// documentation.
	pub params: Vec<u8>,
}
impl Predicate for AtRest {}

/// Compact representation of a state transition where input objects are
/// only referenced by their digest, and the transition object only carries
/// new data. This representation assumes that the user of the transition
/// has access to some store that can provide the expanded versions of the
/// input objects.
///
/// This representation is used for on-chain extrinsics invocation, or
/// when the input object is well-anchored (such as commited to chain),
/// or is a result of a previous transition.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub struct Compact;
impl Repr for Compact {
	type Data = Vec<u8>;
	type InputObject = Digest;
	type Predicate = AtRest;
}

/// This is a representation of a state transition where all input objects
/// are fully expanded and available in the transition object. This
/// representation is used when predicates are evaluated, by blockchain
/// explorers or when the input objects are not available yet.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub struct Expanded;
impl Repr for Expanded {
	type Data = Vec<u8>;
	type InputObject = AsObject<Self>;
	type Predicate = AtRest;
}
