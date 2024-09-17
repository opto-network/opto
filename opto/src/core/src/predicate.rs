use {
	derive_more::derive::{Display, From, Into},
	scale::{Decode, Encode},
	scale_info::TypeInfo,
};

pub trait Predicate {}

/// A unique identifier for a predicate.
///
/// Predicates are identified on chain by an unique u32. Those identifiers
/// are assigned during the installation of the predicate on chain. The author
/// of the predicate can choose the identifier, but it must not be already
/// taken.
///
/// Some of the predicate ids are installed at genesis time.
#[derive(
	Copy,
	Clone,
	Hash,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Debug,
	Encode,
	Decode,
	TypeInfo,
	Display,
	Into,
	From,
)]
pub struct PredicateId(pub u32);
