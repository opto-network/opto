use {
	nft::NftIdentity,
	opto::{
		futures::StreamExt,
		query::{Capture, ObjectPattern, TransitionPattern},
		Client,
		Digest,
		Hashable,
		StreamingClient,
	},
	scale::Decode,
	std::collections::HashMap,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("NFT-Solver example");

	// connect to an RPC node
	let client = Client::new().await?;

	println!("Listening for state transitions...");

	// create a stream of state transitions
	let mut stream = client.transitions();

	let interesting_nft = "BAYC'2025";
	let interesting_mint = Digest::compute(interesting_nft.as_bytes());

	let bayc_nft_pattern = TransitionPattern::default().output(
		ObjectPattern::default().capture_policy(
			nft::ids::NFT,
			move |nft: nft::NftIdentity| nft.mint == interesting_mint,
			Some("BaycNftIdentity"),
		),
	);

	let mut observed_interesting_objects = HashMap::new();
	while let Some(Ok(transition)) = stream.next().await {
		for capture in bayc_nft_pattern.matches(&transition) {
			if let (Some("BaycNftIdentity"), Capture::Policy(object, predicate)) =
				capture
			{
				let identity = NftIdentity::decode(&mut predicate.params.as_slice())?;
				println!("An interesting NFT was produced: {identity:#?}");
				observed_interesting_objects.insert(object.digest(), object.clone());
			}
		}

		// see if any of our previously interesting NFTs were consumed
		if let Some(nft) = transition
			.inputs
			.iter()
			.filter_map(|obj| observed_interesting_objects.remove(&obj.digest()))
			.next()
		{
			println!("An interesting NFT was consumed: {nft:?}");
		}
	}

	Ok(())
}
