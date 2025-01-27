use clap::Parser;

mod opts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let opts = opts::Opts::parse();
	opts.setup_logging_auto();

	#[cfg(not(any(feature = "beacon", feature = "chain")))]
	compile_error!(
		"At least one of the CLI features `chain` or `beacon` must be enabled"
	);

	match opts.command {
		#[cfg(feature = "chain")]
		opts::Command::Chain(opts) => opto_chain::start(opts).await,

		#[cfg(feature = "beacon")]
		opts::Command::Beacon(_opts) => todo!(),
	}
}
