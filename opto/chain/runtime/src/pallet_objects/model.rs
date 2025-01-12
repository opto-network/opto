use {
	crate::interface::AccountId,
	opto_core::Object,
	scale::{Decode, Encode},
	scale_info::TypeInfo,
};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// A timestamp is a 64-bit unsigned integer that represents the number of
/// milliseconds since the Unix epoch.
pub type Timestamp = u64;

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo)]
pub struct Hold {
	pub by: AccountId,
	pub until: Timestamp,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo)]
pub struct ActiveObject {
	/// The total number of copies of the object that are stored.
	/// Each time an object is consumed, this value is decremented by 1.
	/// When the value reaches 0, the object is removed from the storage.
	pub instance_count: u32,

	/// A list of reservations that are currently active for this object.
	pub reservations: Vec<Hold>,

	/// The contents of the object.
	pub content: Object,
}
