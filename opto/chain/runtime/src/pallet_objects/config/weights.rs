use frame::prelude::Weight;

pub trait WeightInfo {
	fn wrap() -> Weight;
	fn unwrap() -> Weight;
	fn install() -> Weight;
}

pub struct SubstrateWeightInfo;
impl WeightInfo for SubstrateWeightInfo {
	fn wrap() -> Weight {
		Weight::from_all(1)
	}

	fn unwrap() -> Weight {
		Weight::from_all(1)
	}

	fn install() -> Weight {
		Weight::from_all(1)
	}
}
