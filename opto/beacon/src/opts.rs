use {clap::Args, core::net::SocketAddr, opto_p2p::multiaddr::Multiaddr};

#[derive(Debug, Args)]
pub struct BeaconOpts {
	/// Address to listen on for incoming connections.
	#[clap(short, long, default_values = ["0.0.0.0:2000", "[::]:2000"])]
	listen_addr: Vec<SocketAddr>,

	/// List of bootstrap nodes to connect to.
	#[clap(short, long)]
	bootstrap: Vec<Multiaddr>,

	/// Don't use default bootstrap nodes.
	#[clap(long, default_value_t = false)]
	no_default_bootstrap: bool,
}
