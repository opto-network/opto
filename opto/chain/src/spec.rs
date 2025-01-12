use {
	opto_core::Digest,
	runtime::{BalancesConfig, ObjectsConfig, SudoConfig, WASM_BINARY},
	sc_service::{ChainType, Properties},
	serde_json::{json, Value},
	sp_keyring::AccountKeyring,
};

/// This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

fn props() -> Properties {
	let mut properties = Properties::new();
	properties.insert("tokenDecimals".to_string(), 4.into());
	properties.insert("tokenSymbol".to_string(), "OPTO".into());
	properties
}

pub fn localdev_config() -> Result<ChainSpec, String> {
	Ok(
		ChainSpec::builder(
			WASM_BINARY.expect("Development wasm not available"),
			Default::default(),
		)
		.with_name("Development")
		.with_id("dev")
		.with_protocol_id("optonet")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(devnet_genesis())
		.with_properties(props())
		.build(),
	)
}

pub fn devnet_config() -> Result<ChainSpec, String> {
	Ok(
		ChainSpec::builder(
			WASM_BINARY.expect("Development wasm not available"),
			Default::default(),
		)
		.with_name("Development")
		.with_id("dev")
		.with_protocol_id("optonet")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(devnet_genesis())
		.with_properties(props())
		.build(),
	)
}

fn devnet_genesis() -> Value {
	use {
		frame::traits::Get,
		runtime::interface::{Balance, MinimumBalance},
	};
	let endowment = <MinimumBalance as Get<Balance>>::get().max(1) * 100000000000;
	let balances = AccountKeyring::iter()
		.map(|a| (a.to_account_id(), endowment))
		.collect::<Vec<_>>();

	json!({
		"balances": BalancesConfig { balances },
		"sudo": SudoConfig { key: Some(AccountKeyring::Alice.to_account_id()) },
		"objects": ObjectsConfig {
			// if this file is not present during build and the build fails, ensure
			// that stdpred is built with archive feature first. This will generate the
			// file in the target directory.
			// `cargo build -p opto-stdpred --release --features=archive`
			stdpred: include_bytes!("../../../target/opto-stdpred.car").to_vec(),
			objects: vec![],
			vrf_seed: Some(Digest::compute(b"opto")),
			phantom: core::marker::PhantomData
		},
		"assets": {
			"assets": vec![
				 (1, AccountKeyring::Alice.to_account_id(), true, 10000),
				 (2, AccountKeyring::Alice.to_account_id(), true, 10000),
			],
			"metadata": vec![
				 (1, b"Tether USD".to_vec(), b"USDT".to_vec(), 4),
				 (2, b"Wrapped Ether".to_vec(), b"WETH".to_vec(), 4),
			],
			"accounts":
					AccountKeyring::iter().map(|a| (1, a.to_account_id(), 700000000))
				.chain(
					AccountKeyring::iter().map(|a| (2, a.to_account_id(), 700000000))
				).collect::<Vec<_>>()
		},
	})
}
