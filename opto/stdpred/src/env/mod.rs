pub mod after;
pub mod before;

pub use {
	after::{after_block, after_time},
	before::{before_block, before_time},
};

pub mod ids {
	pub use super::{
		after::{after_block_id as after_block, after_time_id as after_time},
		before::{before_block_id as before_block, before_time_id as before_time},
	};
}
