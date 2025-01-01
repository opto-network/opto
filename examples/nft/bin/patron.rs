use {
	nft::Mint,
	opto::*,
	rand::random,
	signer::sr25519::dev::{self, alice, bob, charlie},
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

	for _ in 0..5 {
		mint_for(&client, &mint, "Bob", bob().public_key().to_account_id()).await?;
	}

	for _ in 0..5 {
		mint_for(
			&client,
			&mint,
			"Charlie",
			charlie().public_key().to_account_id(),
		)
		.await?;
	}

	Ok(())
}

async fn mint_for(
	client: &Client,
	mint: &Mint,
	name: &str,
	account_id: AccountId32,
) -> anyhow::Result<()> {
	let serial_number = random::<u16>();
	println!("Minting NFT#{serial_number} for {name}..");
	let nft = mint_nft(&client, mint, serial_number, &account_id).await?;
	println!("Minted NFT for {name} successfully. NFT object: {nft:#?}");
	Ok(())
}

async fn mint_nft(
	client: &Client,
	mint: &Mint,
	serial_number: u16,
	identity: &AccountId32,
) -> Result<Object, nft::Error> {
	let data = format!(
		"hair={},shirt={},eyes={},ears={}",
		serial_number + 1,
		serial_number + 2,
		serial_number + 3,
		serial_number + 4
	);

	let tx = mint
		.issue(Digest::compute(&serial_number.to_le_bytes()[..]))
		.recipient(&identity)
		.data(data.as_bytes().to_vec())
		.transition()
		.set_nonces()
		.sign(&alice());

	let nft = tx.outputs[1].clone();
	client.apply(&alice(), vec![tx]).await?;

	Ok(nft)
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
