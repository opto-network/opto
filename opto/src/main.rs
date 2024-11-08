use {clap::Parser, core::future};

mod opts;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let opts = opts::Opts::parse();
	opts.setup_logging_auto();

	match opts.command {
		#[cfg(feature = "chain")]
		opts::Command::Chain(opts) => {
			opto_chain::SubstrateChain::start(opts).await?;
			future::pending::<()>().await;
		}

		#[cfg(feature = "beacon")]
		opts::Command::Beacon(_opts) => todo!(),
	}

	Ok(())
}
