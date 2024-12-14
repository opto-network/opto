use {
	super::*,
	futures::Stream,
	model::{objects::pallet::Event as ObjectsEvent, Event},
	opto_core::{repr::Compact, Transition},
	std::sync::Arc,
	subxt::{OnlineClient, SubstrateConfig},
	tokio::sync::mpsc::unbounded_channel,
	tokio_stream::wrappers::UnboundedReceiverStream,
};

impl StreamingClient for Client {
	type Error = subxt::Error;

	fn transitions(
		&self,
	) -> impl Stream<Item = Result<Transition<Compact>, Self::Error>> {
		let (tx, rx) = unbounded_channel();
		let client = Arc::clone(&self.client);
		tokio::spawn(async move {
			if let Err(e) = recv_loop(client, tx.clone()).await {
				let _ = tx.send(Err(e));
			}
		});

		UnboundedReceiverStream::new(rx)
	}
}

async fn recv_loop<E>(
	client: Arc<OnlineClient<SubstrateConfig>>,
	tx: tokio::sync::mpsc::UnboundedSender<Result<Transition<Compact>, E>>,
) -> Result<(), subxt::Error> {
	let mut subscription = client.blocks().subscribe_finalized().await?;
	while let Some(Ok(block)) = subscription.next().await {
		for event in block.events().await?.iter() {
			match event?.as_root_event::<Event>()? {
				Event::Objects(ObjectsEvent::StateTransitioned { transition }) => {
					let _ = tx.send(Ok(transition));
				}
				_ => continue,
			}
		}
	}
	Ok(())
}
