//! This example wraps assets in an object, performs a state transition that
//! represents a transfer of the wrapped assets, and then unwraps the assets by
//! the new owner.

use {conventions::CoinTransfer, opto::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let amount = std::env::args()
		.nth(1)
		.unwrap_or("15000000".to_string())
		.parse::<u64>()?;

	let asset_id = std::env::args()
		.nth(2)
		.unwrap_or("1".to_string())
		.parse::<u32>()?;

	let client = opto::Client::new().await?;

	let alice = opto::signer::sr25519::dev::alice();
	let charlie = opto::signer::sr25519::dev::charlie();

	let alice_account_id = alice.public_key().to_account_id();
	let charlie_account_id = charlie.public_key().to_account_id();

	println!(
		"Alice USDT balance: {}",
		client.asset_balance(&alice_account_id, asset_id).await?
	);
	println!(
		"Charlie USDT balance: {}",
		client.asset_balance(&charlie_account_id, asset_id).await?
	);

	let wrapped = client.wrap(&alice, asset_id, amount, None).await?;
	println!("Wrapped {}: {:#?}", wrapped.digest(), wrapped);

	println!(
		"Alice USDT balance: {}",
		client.asset_balance(&alice_account_id, asset_id).await?
	);
	println!(
		"Charlie USDT balance: {}",
		client.asset_balance(&charlie_account_id, asset_id).await?
	);

	let transition = CoinTransfer::with_inputs([wrapped])?
		.add_beneficiary(&charlie_account_id, amount / 3)?
		.transition()?
		.sign(&alice);

	let output_digest = transition.outputs[0].digest();

	println!("Transition: {:#?}", transition);
	client.apply(&charlie, vec![transition]).await?;

	client.unwrap(&charlie, &output_digest).await?;

	println!(
		"Alice USDT balance: {}",
		client.asset_balance(&alice_account_id, asset_id).await?
	);
	println!(
		"Charlie USDT balance: {}",
		client.asset_balance(&charlie_account_id, asset_id).await?
	);

	Ok(())
}
