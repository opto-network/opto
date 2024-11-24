#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let client = opto::Client::new().await?;

	let alice = opto::signer::sr25519::dev::alice();
	let charlie = opto::signer::sr25519::dev::charlie();
	Ok(())
}
