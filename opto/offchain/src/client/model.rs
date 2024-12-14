include!(concat!(env!("OUT_DIR"), "/model.rs"));

pub use model::{
	runtime_types::{
		frame_system as system,
		opto_chain_runtime::pallet_objects as objects,
	},
	*,
};
