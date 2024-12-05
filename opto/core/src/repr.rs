use {
	crate::{
		env::Environment,
		eval::{Context, InUse},
		expression::Expression,
		object::Object,
		AtRest,
		Digest,
		Transition,
	},
	alloc::vec::Vec,
	scale::{Decode, Encode},
	scale_decode::DecodeAsType,
	scale_encode::EncodeAsType,
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
pub trait Repr
where
	Self: Sized,
{
	type Data: AsRef<[u8]>;
	type InputObject;
	type Predicate;
}

pub type AsInput<R> = <R as Repr>::InputObject;
pub type AsPredicate<R> = <R as Repr>::Predicate;
pub type AsObject<R> = Object<<R as Repr>::Predicate, <R as Repr>::Data>;
pub type AsExpression<R> = Expression<<R as Repr>::Predicate>;

/// Compact representation of a state transition where input objects are
/// only referenced by their digest, and the transition object only carries
/// new data. This representation assumes that the user of the transition
/// has access to some store that can provide the expanded versions of the
/// input objects.
///
/// This representation is used for on-chain extrinsics invocation, or
/// when the input object is well-anchored (such as commited to chain),
/// or is a result of a previous transition.
#[derive(
	Debug, Clone, Encode, Decode, TypeInfo, PartialEq, EncodeAsType, DecodeAsType,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(
	Debug, Clone, Encode, Decode, TypeInfo, PartialEq, EncodeAsType, DecodeAsType,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Expanded;
impl Repr for Expanded {
	type Data = Vec<u8>;
	type InputObject = AsObject<Self>;
	type Predicate = AtRest;
}

/// This is a representation of a state transition where all input objects
/// are fully expanded and available in the transition object. This
/// representation is used when predicates are evaluated, and is always tied
/// to an underlying machine that is executing the transition.
///
/// This representation requires a reference to the expanded representation
/// during evaluation. This representation is not serializable, clonable,
/// copiable or comparable.
pub struct Executable<'a, F, E: Environment + 'a>(
	core::marker::PhantomData<(&'a F, E)>,
)
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool;

impl<'a, F, E: Environment> Repr for Executable<'a, F, E>
where
	F: FnOnce(Context<'a, E>, &'a Transition<Expanded>, &'a [u8]) -> bool,
{
	type Data = &'a [u8];
	type InputObject = AsObject<Self>;
	type Predicate = InUse<'a, F, E>;
}
