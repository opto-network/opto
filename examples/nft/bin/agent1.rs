use {
	futures::StreamExt,
	nft::NftIdentity,
	opto::*,
	signer::sr25519::{
		dev::{bob, charlie},
		Keypair,
	},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("NFT-Solver example");

	let keypair = match std::env::args().nth(1) {
		Some(arg) if arg.to_lowercase() == "bob" => bob(),
		Some(arg) if arg.to_lowercase() == "charlie" => charlie(),
		_ => {
			println!("Usage: {} bob|charlie", std::env::args().next().unwrap());
			return Ok(());
		}
	};

	// connect to an RPC node
	let client = Client::new().await?;

	// open a stream of state transitions
	let mut stream = client.transitions();
	println!("Listening for state transitions...");

	// we're looking for state transition that output any NFTs
	// of the interesting mint that are unlocked by me,
	let my_nft_pattern = my_nfts_pattern(&keypair);

	// we're looking for state transitions that create NFTs for sale
	let nft_for_sale_pattern = nfts_for_sale_pattern();

	while let Some(Ok(transition)) = stream.next().await {
		for capture in my_nft_pattern.capture(&transition) {
			let nft = capture
				.as_output()
				.expect("The transition pattern will only capture output objects");

			let identity = nft
				.get::<nft::NftIdentity>("identity")
				.expect("without identity the pattern won't match");

			println!(
				"New BAYC NFT: 0x{:?} unlocked by {:#?}",
				hex::encode(&identity.tag),
				nft.object.unlock
			);

			put_nft_for_sale(&client, &nft.object, &keypair).await?;
		}

		for capture in nft_for_sale_pattern.capture(&transition) {
			let offer = capture
				.as_output()
				.expect("The transition pattern will only capture output objects");

			let ask = offer
				.get::<ObjectsSetPattern<Cold>>("ask")
				.expect("otherwise the pattern won't match");

			println!("New BAYC NFT for sale: {:#?}", ask);
		}
	}

	Ok(())
}

fn nfts_for_sale_pattern() -> TransitionPattern {
	// we're interested in BAYC NFTs
	let interesting_mint = Digest::compute(b"BAYC'2025");

	TransitionPattern::default().output(
		ObjectPattern::named("BaycNftOffer")
			.policies(nft::ids::NFT.named("identity").with_params(
				move |params: NftIdentity| params.mint == interesting_mint,
			))
			.unlock(
				stdpred::ids::OUTPUT
					.named("ask")
					.with_params(|_: ObjectsSetPattern<Cold>| true),
			),
	)
}

fn my_nfts_pattern(keypair: &Keypair) -> TransitionPattern {
	// we're interested in BAYC NFTs
	let interesting_mint = Digest::compute(b"BAYC'2025");
	let account_id = keypair.public_key().to_account_id();

	TransitionPattern::default().output(
		ObjectPattern::named("BaycNft")
			.policies(nft::ids::NFT.named("identity").with_params(
				move |params: NftIdentity| params.mint == interesting_mint,
			))
			.unlock(
				stdpred::ids::SR25519
					.with_params(move |value: AccountId32| value == account_id),
			),
	)
}

async fn put_nft_for_sale(
	client: &Client,
	nft: &Object,
	me: &Keypair,
) -> anyhow::Result<()> {
	let eth_price = get_current_eth_price().await;
	let nft_price = eth_price / 10;

	let my_account_id = me.public_key().to_account_id();

	let expectation = ObjectsSetPattern::<Cold>::fuzzy().must_include(
		ObjectPattern::named("ask")
			.policies(stdpred::ids::COIN.with_params((..).equals(1)))
			.unlock(stdpred::ids::SR25519.with_params((..).equals(my_account_id)))
			.data((..).greater_than_or_equals(nft_price)),
	);

	let intent = Object {
		policies: nft.policies.clone(),
		unlock: stdpred::ids::OUTPUT.params(&expectation).into(),
		data: nft.data.clone(),
	};

	println!("Putting NFT for sale at {nft_price} USDT: {expectation:#?}");

	let transition = Transition {
		inputs: vec![nft.digest()],
		ephemerals: vec![],
		outputs: vec![intent],
	}
	.set_nonces()
	.sign(me);

	client.apply(me, vec![transition]).await?;

	Ok(())
}

async fn get_current_eth_price() -> u64 {
	// this is a placeholder for a real implementation
	4200000
}
