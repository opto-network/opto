use {
	example_nft_predicate::NftIdentity,
	opto::*,
	scale::Encode,
	signer::sr25519::{dev, Keypair},
	stdpred::crypto::sr25519::TransitionExt,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("NFT-Patron");
	let client = opto::Client::new().await?;
	install_predicates(&client).await?;

	// create mint object
	let meme_mint_object = create_mint(&client, b"meme1", &dev::alice()).await?;
	println!("created NFT mint object: {meme_mint_object:#?}");

	let charlies_nft = mint_nft(
		&client,
		&meme_mint_object,
		&dev::charlie().public_key().to_account_id(),
		&dev::alice(),
	)
	.await?;

	println!("minted NFT for Charlie: {charlies_nft:#?}");

	let daves_nft = mint_nft(
		&client,
		&meme_mint_object,
		&dev::dave().public_key().to_account_id(),
		&dev::alice(),
	)
	.await?;

	println!("minted NFT for Dave: {daves_nft:#?}");

	Ok(())
}

async fn install_predicates(client: &opto::Client) -> anyhow::Result<()> {
	let id = example_nft_predicate::ids::NFT;
	let predicate = opto::storage().objects().predicates(id);
	if client.storage().await?.fetch(&predicate).await?.is_none() {
		println!("NFT Predicates not installed on chain, installing now");
		client
			.install(
				&opto::signer::sr25519::dev::alice(),
				include_bytes!("../../../target/example-nft.car").to_vec(),
			)
			.await?;
		println!("Installed NFT Predicates");
	} else {
		println!("NFT Predicates already installed");
	}

	Ok(())
}

async fn create_mint(
	client: &opto::Client,
	identity: &[u8],
	signer: &Keypair,
) -> anyhow::Result<Object> {
	let account_id = signer.public_key().to_account_id();
	let mint_object = Object {
		policies: [
			Predicate {
				id: example_nft_predicate::ids::NFT_MINT,
				params: Digest::compute(identity).to_vec(),
			},
			Predicate {
				id: stdpred::ids::UNIQUE,
				params: Digest::compute(identity).to_vec(),
			},
		]
		.to_vec(),
		unlock: Expression::signature(&account_id),
		data: format!("{:?} by {:?}", identity, account_id).into_bytes(),
	};

	let transition = Transition {
		inputs: vec![],
		ephemerals: vec![],
		outputs: vec![mint_object.clone()],
	};

	client.apply(signer, vec![transition]).await?;

	Ok(mint_object)
}

async fn mint_nft(
	client: &opto::Client,
	mint_object: &Object,
	recipient: &AccountId32,
	mint_owner: &Keypair,
) -> anyhow::Result<Object> {
	let mint_identity: Digest = mint_object
		.policies
		.iter()
		.find(|p| p.id == example_nft_predicate::ids::NFT_MINT)
		.expect("mint object must have NFT_MINT policy")
		.params
		.as_slice()
		.try_into()
		.expect("NFT mint has invalid identity");

	// this is the new NFT output object.
	let new_nft = Object {
		policies: vec![
			Predicate {
				id: example_nft_predicate::ids::NFT,
				params: NftIdentity {
					mint: mint_identity,
					tag: Digest::compute(recipient.as_ref()),
					mutable: false,
				}
				.encode(),
			},
			Predicate {
				id: stdpred::ids::UNIQUE,
				params: Digest::compute_concat(&[
					mint_identity.as_ref(),
					Digest::compute(recipient.as_ref()).as_ref(),
				])
				.to_vec(),
			},
		],
		unlock: Expression::signature(recipient),
		data: format!("NFT by {:?}", recipient).into_bytes(),
	};

	let mut transition = Transition {
		inputs: vec![mint_object.digest()],
		ephemerals: vec![],
		outputs: vec![new_nft.clone(), mint_object.clone()],
	};

	transition.sign_with_sr25519(mint_owner);

	client.apply(mint_owner, vec![transition]).await?;

	Ok(new_nft)
}
