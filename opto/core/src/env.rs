use {crate::Digest, core::time::Duration};

/// Represents the execution environment of the state transition VM.
/// When executed on chain this will have a snaphot of the state of the
/// block in which a transition is contained.
///
/// Clients can synchronize this using the off-chain SDK. When predicates
/// are evaluated, they will be evaluated with an instance of this trait
/// and will always have access to it.
pub trait Environment {
	/// The current block number.
	fn block_number(&self) -> u32;

	/// The length of the history that the chain state holds
	/// for timestamps and VRF values. Only the latest N values
	/// are available for callers.
	fn history_len(&self) -> u32;

	/// The time at which a block was produced.
	/// By default the chain state holds about 24h of history.
	fn time_at(&self, block: u32) -> Option<Duration>;

	/// The current value of the Verifiable Random Function.
	/// In the current implementation on-chain this is calculated by
	/// By default the chain state holds about 24h of history.
	fn vrf_at(&self, block: u32) -> Option<Digest>;

	/// The current value of the Verifiable Random Function for the current block.
	fn vrf_now(&self) -> Digest {
		self
			.vrf_at(self.block_number())
			.expect("VRF value must always be available for the current block")
	}

	/// The current time.
	fn time_now(&self) -> Duration {
		self
			.time_at(self.block_number())
			.expect("Time value must always be available for the current block")
	}
}

#[cfg(any(test, feature = "test"))]
#[derive(Debug, Default, Clone)]
pub struct StaticEnvironment {
	pub block_number: u32,
	pub vrfs: alloc::collections::BTreeMap<u32, Digest>,
	pub times: alloc::collections::BTreeMap<u32, Duration>,
}

#[cfg(any(test, feature = "test"))]
impl Environment for StaticEnvironment {
	fn block_number(&self) -> u32 {
		self.block_number
	}

	fn vrf_at(&self, block: u32) -> Option<Digest> {
		self.vrfs.get(&block).cloned()
	}

	fn time_at(&self, block: u32) -> Option<Duration> {
		self.times.get(&block).cloned()
	}

	fn history_len(&self) -> u32 {
		0
	}
}
