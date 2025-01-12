use frame::prelude::Weight;

pub trait WeightInfo {
	fn wrap() -> Weight;
	fn unwrap() -> Weight;
	fn apply() -> Weight;
	fn reserve() -> Weight;
	fn install() -> Weight;
	fn vrf_init() -> Weight;
	fn timestamp_init() -> Weight;
	fn reclaim_expired_reservations() -> Weight;
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

	fn apply() -> Weight {
		Weight::from_all(1)
	}

	fn reserve() -> Weight {
		Weight::from_all(1)
	}

	fn vrf_init() -> Weight {
		Weight::from_all(1)
	}

	fn timestamp_init() -> Weight {
		Weight::from_all(1)
	}

	fn reclaim_expired_reservations() -> Weight {
		Weight::from_all(1)
	}
}
