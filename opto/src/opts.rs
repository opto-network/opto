use {
	clap::{ArgAction, Args, Subcommand},
	env_logger::{TimestampPrecision, WriteStyle},
	std::path::PathBuf,
};

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Opts {
	#[clap(subcommand)]
	pub command: Command,

	#[clap(flatten)]
	pub global: GlobalOpts,
}

#[derive(Debug, Args)]
pub struct GlobalOpts {
	/// Verbose logging (-vv for extra verbosity)
	#[clap(short, global = true, action = ArgAction::Count)]
	pub verbose: u8,

	/// Path to the workspace directory
	#[clap(long, global = true)]
	pub workspace: Option<PathBuf>,
}

/// The various modes the node can run in.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum Command {
	/// Starts a blockchain validator node.
	#[cfg(feature = "chain")]
	Chain(opto_chain::ChainOpts),

	/// Beacon mode supports P2P bootstrap nodes, IPFS cache nodes, and resource
	/// indexers, while enhancing message relaying and redundancy in the compute
	/// network.
	#[cfg(feature = "beacon")]
	Beacon(opto_beacon::BeaconOpts),
}

impl Opts {
	pub fn setup_logging(&self, interactive: bool) {
		env_logger::builder()
			.format_target(false)
			.format_level(!interactive)
			.format_module_path(false)
			.format_timestamp(match interactive {
				true => None,
				false => Some(TimestampPrecision::Seconds),
			})
			.write_style(WriteStyle::Auto)
			.filter_module("zbus", log::LevelFilter::Warn)
			.filter_module("wasmtime", log::LevelFilter::Info)
			.filter_module("wasmtime_environ", log::LevelFilter::Info)
			.filter_module("cranelift_wasm", log::LevelFilter::Info)
			.filter_module("cranelift_codegen", log::LevelFilter::Info)
			.filter_level(match self.global.verbose {
				0 => log::LevelFilter::Info,
				1 => log::LevelFilter::Debug,
				_ => log::LevelFilter::Trace,
			})
			.init();
	}

	pub fn setup_logging_auto(&self) {
		match &self.command {
			#[cfg(feature = "chain")]
			Command::Chain(cmd) => self.setup_logging(cmd.is_interactive()),

			#[cfg(feature = "beacon")]
			Command::Beacon(cmd) => self.setup_logging(cmd.is_interactive()),

			#[cfg(not(any(feature = "chain", feature = "beacon")))]
			_ => self.setup_logging(false),
		}
	}
}
