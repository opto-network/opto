use {
	super::{rpc, ChainOpts, Event},
	frame_system::EventRecord,
	futures::StreamExt,
	runtime::{interface::OpaqueBlock as Block, pallet_objects, RuntimeApi},
	sc_cli::CliConfiguration,
	sc_network::NetworkBackend,
	sc_service::TaskManager,
	sc_telemetry::TelemetryWorker,
	sc_utils::mpsc::tracing_unbounded,
	scale::Decode,
	sp_consensus::StateBackend,
	sp_core::twox_128,
	sp_io::SubstrateHostFunctions,
	sp_runtime::traits::Block as BlockT,
	std::sync::Arc,
	tokio::sync::mpsc::UnboundedSender,
};

pub async fn start_chain(
	opts: &ChainOpts,
	events_tx: UnboundedSender<Event>,
) -> anyhow::Result<TaskManager> {
	let tokio = tokio::runtime::Handle::current();
	let config = opts.cmd.create_configuration(opts, tokio)?;

	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor =
		sc_service::new_wasm_executor::<SubstrateHostFunctions>(&config);

	let (client, backend, keystore_container, mut task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let (import_tx, import_rx) = tracing_unbounded("block_import", 1024);
	client.import_notification_sinks().lock().push(import_tx);

	let client_ref = Arc::clone(&client);
	tokio::spawn(async move {
		let mut import_rx = import_rx;
		while let Some(import) = import_rx.next().await {
			if import.origin == sp_consensus::BlockOrigin::Own {
				let state = client_ref.state_at(import.hash).unwrap();
				let events_key = get_events_storage_key();
				if let Some(events_storage) = state.storage(&events_key).unwrap() {
					let events_vec: Vec<
						EventRecord<runtime::RuntimeEvent, sp_core::hash::H256>,
					> = Decode::decode(&mut &events_storage[..]).unwrap();
					for event in events_vec {
						if let runtime::RuntimeEvent::Objects(event) = event.event {
							match event {
								pallet_objects::Event::StateTransitioned { transition } => {
									let _ = events_tx.send(Event::StateTransitioned(transition));
								}
								pallet_objects::Event::PredicateInstalled { id } => {
									let _ = events_tx.send(Event::PredicateInstalled(id));
								}
								_ => unreachable!(),
							}
						}
					}
				}
			}
		}
	});

	let mut telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let import_queue = sc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	type Network = sc_network::NetworkWorker<Block, <Block as BlockT>::Hash>;

	let net_config = sc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as BlockT>::Hash,
		Network,
	>::new(&config.network);

	let metrics = Network::register_notification_metrics(
		config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);

	let (
		network,
		system_rpc_tx,
		tx_handler_controller,
		network_starter,
		sync_service,
	) = sc_service::build_network(sc_service::BuildNetworkParams {
		config: &config,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		spawn_handle: task_manager.spawn_handle(),
		import_queue,
		net_config,
		block_announce_validator_builder: None,
		warp_sync_params: None,
		block_relay: None,
		metrics,
	})?;

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
			};
			super::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let prometheus_registry = config.prometheus_registry().cloned();

	let spawn_params = sc_service::SpawnTasksParams {
		network,
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend,
		system_rpc_tx,
		tx_handler_controller,
		sync_service,
		config,
		telemetry: telemetry.as_mut(),
	};

	let _rpc_handlers = sc_service::spawn_tasks(spawn_params)?;

	let proposer = sc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		prometheus_registry.as_ref(),
		telemetry.as_ref().map(|x| x.handle()),
	);

	let block_time = std::time::Duration::from_secs(6);
	let (mut sink, commands_stream) = futures::channel::mpsc::channel(1024);
	task_manager
		.spawn_handle()
		.spawn("block_authoring", None, async move {
			loop {
				futures_timer::Delay::new(block_time).await;
				sink
					.try_send(sc_consensus_manual_seal::EngineCommand::SealNewBlock {
						create_empty: true,
						finalize: true,
						parent_hash: None,
						sender: None,
					})
					.unwrap();
			}
		});

	let params = sc_consensus_manual_seal::ManualSealParams {
		block_import: client.clone(),
		env: proposer,
		client,
		pool: transaction_pool,
		select_chain,
		commands_stream: Box::pin(commands_stream),
		consensus_data_provider: None,
		create_inherent_data_providers: move |_, ()| async move {
			Ok(sp_timestamp::InherentDataProvider::from_system_time())
		},
	};
	let authorship_future = sc_consensus_manual_seal::run_manual_seal(params);

	task_manager.spawn_essential_handle().spawn_blocking(
		"manual-seal",
		None,
		authorship_future,
	);

	network_starter.start_network();

	Ok(task_manager)
}

fn get_events_storage_key() -> Vec<u8> {
	let pallet_name = b"System";
	let storage_item_name = b"Events";
	let mut key = twox_128(pallet_name).to_vec();
	key.extend_from_slice(&twox_128(storage_item_name)[..]);
	key
}
