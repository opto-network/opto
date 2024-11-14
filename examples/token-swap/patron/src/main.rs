use opto::{Hashable, MutatingClient, ReadOnlyClient};

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

	let alice = opto::sr25519::dev::alice();
	let charlie = opto::sr25519::dev::charlie();

	let account_id = alice.public_key().to_account_id();
	let usdt_balance = client.asset_balance(&account_id, asset_id).await?;
	let native_balance = client.native_balance(&account_id).await?;

	println!("Alice USDT balance: {}", usdt_balance);
	println!("Alice native balance: {}", native_balance);

	let wrapped = client.wrap(&alice, asset_id, amount, None).await?;
	println!("Wrapped {}: {:?}", wrapped.digest(), wrapped);

	let usdt_balance = client.asset_balance(&account_id, asset_id).await?;
	let native_balance = client.native_balance(&account_id).await?;

	println!("Alice USDT balance: {}", usdt_balance);
	println!("Alice native balance: {}", native_balance);

	println!("Unwrapping...");
	client.unwrap(&alice, &wrapped.digest()).await?;

	println!("Alice USDT balance: {}", usdt_balance);
	println!("Alice native balance: {}", native_balance);

	assert!(client.object(&wrapped.digest()).await?.is_none());

	Ok(())
}
