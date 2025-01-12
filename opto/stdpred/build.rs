fn main() {
	#[cfg(feature = "archive")]
	opto_onchain_builder::build_in_release();
}
