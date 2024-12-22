use opto::{futures::StreamExt, StreamingClient as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let client = opto::Client::new().await?;

	let mut stream = client.transitions();
	while let Some(interesting) = stream.next().await {
		println!("Transition: {interesting:?}");
		println!();
	}

	Ok(())
}
