use {
	ext::*,
	nft::Mint,
	opto::*,
	rand::random,
	signer::sr25519::dev::{self, alice, charlie},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("NFT devx example");

	const NFT_NAME: &str = "BAYC'2025";

	// connect to an RPC node
	let client = Client::new().await?;

	println!("installing predicates");
	install_predicates(&client).await?;

	println!("Fetching {NFT_NAME} NFT mint object");
	let identity = Digest::compute(NFT_NAME.as_bytes());
	let mint = open_or_create_mint(&client, identity).await?;

	println!("Mint object: {mint:#?}");

	let nftid = random::<u16>().to_string();

	println!("Minting {NFT_NAME} #{nftid} for charlie...");
	let tx = mint
		.issue(Digest::compute(nftid.as_bytes()))
		.recipient(&charlie().public_key().to_account_id())
		.data(b"hair=1,shirt=28,eyes=981,ears=82".to_vec())
		.transition()
		.set_nonces()
		.sign(&alice());

	println!("mint transition: {:#?}", tx);

	let charlies_nft = tx.outputs[0].clone();
	client.apply(&alice(), vec![tx]).await?;
	println!(
		"minted NFT for Charlie successfully. NFT object: {:#?}",
		charlies_nft
	);

	let nftid = random::<u16>().to_string();
	println!("Minting {NFT_NAME} #{nftid} for dave...");
	let tx = mint
		.issue(Digest::compute(nftid.as_bytes()))
		.recipient(&dev::dave().public_key().to_account_id())
		.data(b"hair=1,shirt=28,eyes=981,ears=82".to_vec())
		.transition()
		.set_nonces()
		.sign(&alice());

	println!("mint transition: {:#?}", tx);

	let daves_nft = tx.outputs[0].clone();
	client.apply(&alice(), vec![tx]).await?;

	println!(
		"minted NFT for Dave successfully. NFT object: {:#?}",
		daves_nft
	);

	Ok(())
}

async fn open_or_create_mint(
	client: &Client,
	identity: Digest,
) -> Result<Mint, nft::Error> {
	match Mint::fetch(client, &identity).await {
		Err(nft::Error::MintNotFound) => {
			println!("Mint not found, creating a new one");

			let mint_obj = nft::MintBuilder::new(identity)
				.with_owner(dev::alice().public_key().to_account_id())
				.with_data(b"game1_weapons_class1".to_vec())
				.build();

			let transition = mint_obj.clone().spawn();
			println!("Mint creation transition: {transition:#?}");

			// submit the transition, this will spawn a new mint object
			client.apply(&alice(), vec![transition]).await?;

			// spawning mint object succeeded, use it
			Ok(mint_obj.try_into().expect("built using SDK mint builder"))
		}
		result => result,
	}
}

async fn install_predicates(client: &Client) -> anyhow::Result<()> {
	let binary = include_bytes!("../../../target/example-nft.car").to_vec();

	if client.predicate(nft::ids::NFT).await?.is_none() {
		client.install(&alice(), binary).await?;
	}

	Ok(())
}
