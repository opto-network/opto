use {
	crate::spec,
	clap::{Args, Subcommand},
	sc_cli::{
		BuildSpecCmd,
		CliConfiguration,
		PurgeChainCmd,
		RunCmd,
		SubstrateCli,
	},
};

#[derive(Debug, Args)]
pub struct ChainOpts {
	#[clap(flatten)]
	pub cmd: RunCmd,

	#[clap(subcommand)]
	pub subcommand: Option<SubCommand>,
}

impl ChainOpts {
	pub fn is_interactive(&self) -> bool {
		false
	}
}

#[derive(Debug, Subcommand)]
pub enum SubCommand {
	BuildSpec(BuildSpecCmd),
	Purge(PurgeChainCmd),
}

impl SubstrateCli for ChainOpts {
	fn impl_name() -> String {
		"Opto Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://opto.network".into()
	}

	fn copyright_start_year() -> i32 {
		2024
	}

	fn load_spec(
		&self,
		id: &str,
	) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(spec::localdev_config()?),
			_ => Box::new(spec::devnet_config()?),
		})
	}
}

impl CliConfiguration for ChainOpts {
	fn shared_params(&self) -> &sc_cli::SharedParams {
		self.cmd.shared_params()
	}
}
