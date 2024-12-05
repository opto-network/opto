use opto::{futures::StreamExt, StreamingClient as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let client = opto::Client::new().await?;

	let alice = opto::signer::sr25519::dev::alice();
	let charlie = opto::signer::sr25519::dev::charlie();

	let mut stream = client.transitions();
	while let Some(interesting) = stream.next().await {
		println!("Transition: {interesting:?}");
		println!();
	}

	Ok(())
}
