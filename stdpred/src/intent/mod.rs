pub mod ephemeral;
pub mod input;
pub mod output;

pub use {ephemeral::ephemeral, input::input, output::output};

pub mod ids {
	pub use super::{
		ephemeral::ephemeral_id as ephemeral,
		input::input_id as input,
		output::output_id as output,
	};
}
