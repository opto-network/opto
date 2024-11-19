//! This example wraps assets in an object, performs a state transition that
//! represents a transfer of the wrapped assets, and then unwraps the assets by
//! the new owner.

use opto::{
	repr::Compact,
	CompactTransitionExt as _,
	Hashable,
	MutatingClient,
	ReadOnlyClient,
	Transition,
};

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
	println!("Wrapped {}: {:?}", wrapped.digest(), wrapped);

	println!(
		"Alice USDT balance: {}",
		client.asset_balance(&alice_account_id, asset_id).await?
	);
	println!(
		"Charlie USDT balance: {}",
		client.asset_balance(&charlie_account_id, asset_id).await?
	);

	let wrapped_digest = wrapped.digest();

	// create a state transition that changes the unlock expression
	// of the wrapped asset to a new owner
	let output = opto::Object {
		policies: wrapped.policies,
		unlock: opto::AtRest {
			id: opto::stdpred::crypto::sr25519::sr25519_id,
			params: charlie.public_key().0.to_vec(),
		}
		.into(),
		data: wrapped.data,
	};
	let output_digest = output.digest();

	let mut transition = Transition::<Compact> {
		inputs: vec![wrapped_digest],
		ephemerals: vec![],
		outputs: vec![output],
	};

	transition.sign(&alice);

	println!("Transition: {:?}", transition);
	let (created, destroyed) = client.apply(&charlie, vec![transition]).await?;

	println!("objects created: {created:?}");
	println!("objects destroyed: {destroyed:?}");
	assert_eq!(created, vec![output_digest]);

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
