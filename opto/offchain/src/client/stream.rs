use {
	super::*,
	futures::Stream,
	model::{objects::pallet::Event as ObjectsEvent, Event as RuntimeEvent},
	std::sync::Arc,
	subxt::{OnlineClient, SubstrateConfig},
	tokio::sync::mpsc::{unbounded_channel, UnboundedSender},
	tokio_stream::wrappers::UnboundedReceiverStream,
};

impl StreamingClient for Client {
	type Error = subxt::Error;

	fn events(&self) -> impl Stream<Item = Result<ObjectsEvent, Self::Error>> {
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

async fn recv_loop(
	client: Arc<OnlineClient<SubstrateConfig>>,
	tx: UnboundedSender<Result<ObjectsEvent, subxt::Error>>,
) -> Result<(), subxt::Error>
where
{
	let mut subscription = client.blocks().subscribe_finalized().await?;
	while let Some(Ok(block)) = subscription.next().await {
		for event in block.events().await?.iter() {
			if let RuntimeEvent::Objects(e) =
				event?.as_root_event::<RuntimeEvent>()?
			{
				let _ = tx.send(Ok(e));
			}
		}
	}
	Ok(())
}
