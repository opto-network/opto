mod opts;
mod rpc;
mod spec;
mod substrate;

pub use opts::ChainOpts;
use {
	futures::{FutureExt, Stream},
	log::error,
	opts::SubCommand,
	sc_cli::CliConfiguration,
	tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver},
};

#[derive(Debug)]
pub enum Event {
	ObjectCreated(opto_core::Object),
	ObjectDestroyed(opto_core::Digest),
	PredicateInstalled(opto_core::PredicateId),
	ChainFailed(anyhow::Error),
}

/// This type represents an instance of a running substrate chain.
///
/// Instance of this are used to control and communicate the running instance
/// and it's the gateway API for interacting with the validator node from other
/// components inside this process.
pub struct SubstrateChain {
	events_rx: UnboundedReceiver<Event>,
}

impl SubstrateChain {
	pub async fn start(opts: ChainOpts) -> anyhow::Result<Self> {
		let (events_tx, events_rx) = unbounded_channel();
		let mut task_manager =
			substrate::start_chain(&opts, events_tx.clone()).await?;

		tokio::spawn(async move {
			if let Err(e) = task_manager.future().fuse().await {
				error!("Failed to start chain node: {:?}", e);
				events_tx
					.send(Event::ChainFailed(e.into()))
					.expect("failed to report chain failure");
			}
		});

		Ok(Self { events_rx })
	}
}

impl Stream for SubstrateChain {
	type Item = Event;

	fn poll_next(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context,
	) -> std::task::Poll<Option<Self::Item>> {
		self.events_rx.poll_recv(cx)
	}
}

pub async fn start(opts: ChainOpts) -> anyhow::Result<()> {
	if let Some(ref subcmd) = opts.subcommand {
		match subcmd {
			SubCommand::BuildSpec(cmd) => {
				let config = opts
					.cmd
					.create_configuration(&opts, tokio::runtime::Handle::current())?;
				cmd.run(config.chain_spec, config.network)?;
				return Ok(());
			}
		}
	};

	SubstrateChain::start(opts).await?;
	futures::future::pending::<()>().await;
	Ok(())
}
