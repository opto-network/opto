fn main() {
	#[cfg(feature = "onchain")]
	opto::builder::build_in_release();
}
