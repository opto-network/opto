use {
	alloc::vec::Vec,
	derive_more::derive::{Display, From, Into},
	scale::{Decode, Encode},
	scale_decode::DecodeAsType,
	scale_encode::EncodeAsType,
	scale_info::TypeInfo,
};

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
	Encode,
	Decode,
	TypeInfo,
	Display,
	Into,
	From,
	DecodeAsType,
	EncodeAsType,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PredicateId(pub u32);

impl core::fmt::Debug for PredicateId {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "PredicateId({})", self.0)
	}
}

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
#[derive(
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	TypeInfo,
	EncodeAsType,
	DecodeAsType,
	Hash,
)]
pub struct Predicate {
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

impl core::fmt::Debug for Predicate {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Predicate({}, 0x{})", self.id, hex::encode(&self.params))
	}
}
