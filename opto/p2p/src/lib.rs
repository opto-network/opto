pub use libp2p::{
	identity::{Keypair, PublicKey},
	multiaddr,
	multihash,
	PeerId,
};
use {
	core::{future::Future, task::Poll},
	futures::{FutureExt, Sink, Stream, StreamExt},
	libp2p::{
		gossipsub::{self, Sha256Topic},
		identify,
		kad,
		noise,
		ping,
		swarm::{DialError, NetworkBehaviour, SwarmEvent},
		tcp,
		yamux,
		Multiaddr,
		StreamProtocol,
		Swarm,
		TransportError,
	},
	log::{debug, info},
	opto_core::{Object, Transition},
	scale::{Decode, Encode},
	std::sync::OnceLock,
	thiserror::Error,
	tokio::{
		sync::mpsc::{
			error::SendError,
			unbounded_channel,
			UnboundedReceiver,
			UnboundedSender,
		},
		task::JoinError,
	},
};

#[derive(Debug, Clone)]
pub struct Config {
	pub listen_on: Vec<Multiaddr>,
	pub bootstrap: Vec<Multiaddr>,
	pub idle_timeout: std::time::Duration,
	pub kad_query_timeout: std::time::Duration,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			idle_timeout: std::time::Duration::from_secs(60),
			kad_query_timeout: std::time::Duration::from_secs(5 * 60),
			bootstrap: vec![
				"/dns4/0.beacon.opto.network/tcp/2000",
				"/dns6/0.beacon.opto.network/tcp/2000",
				"/dns4/1.beacon.opto.network/tcp/2000",
				"/dns6/1.beacon.opto.network/tcp/2000",
			]
			.into_iter()
			.map(|addr| addr.parse().unwrap())
			.collect(),

			listen_on: vec!["/ip4/0.0.0.0/tcp/0", "/ip6/::/tcp/0"]
				.into_iter()
				.map(|addr| addr.parse().unwrap())
				.collect(),
		}
	}
}

#[derive(Debug, Error)]
pub enum SetupError {
	#[error("Failed to dial into bootstrap node: {0}")]
	BootstrapDial(#[from] DialError),

	#[error("Failed to setup p2p listener: {0}")]
	ListenOn(#[from] TransportError<std::io::Error>),

	#[error("Failed to setup noise protocol: {0}")]
	NoiseSetup(#[from] noise::Error),

	#[error("Failed to setup gossipsub subscriptions: {0}")]
	Subscription(#[from] gossipsub::SubscriptionError),

	#[error("Io error: {0}")]
	Io(#[from] std::io::Error),
}

#[derive(Encode, Decode, Debug)]
pub enum NetworkEvent {
	StateTransitioned(Transition),
	Disconnected,
}

#[derive(Encode, Decode, Debug)]
pub enum ObjectEvent {
	StateTransitioned(Object),
}

pub struct Network {
	events_rx: UnboundedReceiver<NetworkEvent>,
	events_tx: UnboundedSender<NetworkEvent>,
	join_handle: tokio::task::JoinHandle<()>,
}

#[derive(NetworkBehaviour)]
struct OptoBehaviour {
	ping: ping::Behaviour,
	identify: identify::Behaviour,
	gossipsub: gossipsub::Behaviour,
	kad: kad::Behaviour<kad::store::MemoryStore>,
}

static OBJECTS_TOPIC: OnceLock<Sha256Topic> = OnceLock::new();
const IPFS_PROTO_NAME: StreamProtocol = StreamProtocol::new("/ipfs/kad/1.0.0");

impl OptoBehaviour {
	pub fn new(keypair: &Keypair, config: &Config) -> Self {
		let peer_id = keypair.public().to_peer_id();

		Self {
			ping: ping::Behaviour::new(ping::Config::new()),
			identify: identify::Behaviour::new(identify::Config::new(
				"/ipfs/0.1.0".into(),
				keypair.public(),
			)),
			gossipsub: gossipsub::Behaviour::new(
				gossipsub::MessageAuthenticity::Signed(keypair.clone()),
				gossipsub::ConfigBuilder::default()
					.build()
					.expect("default gossipsub config"),
			)
			.expect("invalid gossipsub config"),
			kad: kad::Behaviour::with_config(
				peer_id,
				kad::store::MemoryStore::new(peer_id),
				kad::Config::new(IPFS_PROTO_NAME)
					.set_query_timeout(config.kad_query_timeout)
					.clone(),
			),
		}
	}
}

impl Network {
	pub fn new(identity: Keypair, config: Config) -> Result<Self, SetupError> {
		let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
			.with_tokio()
			.with_tcp(
				tcp::Config::default(),
				noise::Config::new,
				yamux::Config::default,
			)?
			.with_dns()?
			.with_behaviour(|keypair| OptoBehaviour::new(keypair, &config))
			.expect("behaviour constructor does not fail")
			.with_swarm_config(|c| {
				c.with_idle_connection_timeout(config.idle_timeout)
			})
			.build();

		info!("Local p2p identity: {}", swarm.local_peer_id());

		swarm.behaviour_mut().gossipsub.subscribe(
			OBJECTS_TOPIC.get_or_init(|| Sha256Topic::new("/opto/objects")),
		)?;

		// listen on configured addresses
		for addr in config.listen_on {
			debug!("Listening on: {addr}");
			swarm.listen_on(addr)?;
		}

		// dial into bootstrap nodes
		for addr in config.bootstrap {
			debug!("Dialing into bootstrap node: {addr}");
			swarm.dial(addr)?;
		}

		let (events_tx1, events_rx1) = unbounded_channel();
		let (events_tx2, events_rx2) = unbounded_channel();

		let join_handle = tokio::spawn(swarm_loop(swarm, events_tx1, events_rx2));

		Ok(Self {
			events_rx: events_rx1,
			events_tx: events_tx2,
			join_handle,
		})
	}

	pub fn with_config(config: Config) -> Result<Self, SetupError> {
		Self::new(Keypair::generate_ed25519(), config)
	}
}

impl Default for Network {
	fn default() -> Self {
		Self::new(Keypair::generate_ed25519(), Config::default())
			.expect("invalid hardcoded defaults")
	}
}

async fn swarm_loop(
	swarm: Swarm<OptoBehaviour>,
	events_tx: UnboundedSender<NetworkEvent>,
	events_rx: UnboundedReceiver<NetworkEvent>,
) {
	let mut swarm = swarm;
	let mut events_rx = events_rx;
	let objects_topic = OBJECTS_TOPIC.get().unwrap().clone();

	loop {
		tokio::select! {
			event = events_rx.recv() => {
				let Some(event) = event else {
					continue;
				};

				if let NetworkEvent::Disconnected = event {
					debug!("Requested to disconnect from network");
					break;
				}

				info!("Broadcasting event: {event:?}");

				if let Err(e) = swarm
					.behaviour_mut()
					.gossipsub
					.publish(objects_topic.clone(), event.encode()) {
					log::error!("Failed to publish gossipsub message: {e:?}");
				}
			},
			event = swarm.select_next_some() => match event {
				SwarmEvent::Behaviour(OptoBehaviourEvent::Gossipsub(
					gossipsub::Event::Message { message, .. })) => {
					if message.topic == OBJECTS_TOPIC.get().unwrap().hash() {
						match NetworkEvent::decode(&mut message.data.as_slice()) {
							Ok(event) => {
								let _ = events_tx.send(event);
							}
							Err(e) => {
								log::error!("Failed to decode gossipsub message: {e:?}");
							}
						}
					}
				}
				SwarmEvent::NewListenAddr { address, .. } => {
					log::info!("Listening on: {address}");
					let self_id = *swarm.local_peer_id();
					swarm.behaviour_mut().kad.add_address(&self_id, address);
				}
				SwarmEvent::ExternalAddrConfirmed { address } => {
					log::debug!("External address confirmed: {address}");
					let self_id = *swarm.local_peer_id();
					swarm.behaviour_mut().kad.add_address(&self_id, address);
				}
				SwarmEvent::NewExternalAddrOfPeer { peer_id, address } => {
					log::debug!("New peer: {peer_id} -> {address}");
					swarm.behaviour_mut().kad.add_address(&peer_id, address);
				}
				_ => {}
			}
		}
	}
}

impl Future for Network {
	type Output = Result<(), JoinError>;

	fn poll(
		mut self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> Poll<Self::Output> {
		self.join_handle.poll_unpin(cx)
	}
}

impl Stream for Network {
	type Item = NetworkEvent;

	fn poll_next(
		mut self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Option<Self::Item>> {
		self.events_rx.poll_recv(cx)
	}
}

impl Sink<NetworkEvent> for Network {
	type Error = SendError<NetworkEvent>;

	fn poll_ready(
		self: core::pin::Pin<&mut Self>,
		_: &mut core::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn start_send(
		self: core::pin::Pin<&mut Self>,
		event: NetworkEvent,
	) -> Result<(), Self::Error> {
		self.events_tx.send(event)
	}

	fn poll_flush(
		self: core::pin::Pin<&mut Self>,
		_: &mut core::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn poll_close(
		self: core::pin::Pin<&mut Self>,
		_: &mut core::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}
}
