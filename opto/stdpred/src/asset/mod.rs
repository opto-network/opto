pub mod coin;
pub use coin::coin;

pub mod ids {
	pub use super::coin::coin_id as coin;
}
