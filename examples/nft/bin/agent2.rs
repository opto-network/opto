use {futures::StreamExt, nft::NftIdentity, opto::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("NFT-Solver example");

	// connect to an RPC node
	let client = Client::new().await?;

	// open a stream of state transitions
	let mut stream = client.transitions();
	println!("Listening for state transitions...");

	// we're interested in BAYC NFTs
	let interesting_mint = Digest::compute(b"BAYC'2025");

	// we're looking for state transition that output any NFTs
	// of the interesting mint, the pattern builder is part of the
	// nft module.
	let nft_pattern = TransitionPattern::default().output(
		ObjectPattern::named("BaycNft").policies(
			nft::ids::NFT.named("identity").with_params(
				move |params: NftIdentity| params.mint == interesting_mint,
			),
		),
	);

	// we're looking for state transitions that create mint objects.
	// Mint objects are an output on every mint transition as well because
	// they are reusable. We are however only interested in transitions that
	// output a mint object and no other objects.
	let mint_pattern = TransitionPattern::default().output(
		ObjectsSetPattern::exact().must_include(
			ObjectPattern::named("BaycMint").policies(
				nft::ids::NFT_MINT
					.named("BaycMint")
					.with_params(move |params: Digest| params == interesting_mint),
			),
		),
	);

	while let Some(Ok(transition)) = stream.next().await {
		for capture in nft_pattern.capture(&transition) {
			let nft = capture
				.as_output()
				.expect("The transition pattern will only capture output objects");

			let identity = nft
				.get::<nft::NftIdentity>("identity")
				.expect("without identity the pattern won't match");

			println!(
				"New BAYC NFT: {:?} unlocked by {:#?}",
				hex::encode(&identity.tag),
				nft.object.unlock
			);
		}

		for capture in mint_pattern.capture(&transition) {
			let mint = capture
				.as_output()
				.expect("The transition pattern will only capture output objects");

			println!("New BAYC mint object: {:#?}", mint.object);
		}
	}

	Ok(())
}
