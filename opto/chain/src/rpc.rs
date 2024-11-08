use {
	jsonrpsee::RpcModule,
	runtime::interface::{AccountId, Nonce, OpaqueBlock},
	sc_rpc_api::DenyUnsafe,
	sc_transaction_pool_api::TransactionPool,
	sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata},
	std::sync::Arc,
};

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: Send
		+ Sync
		+ 'static
		+ sp_api::ProvideRuntimeApi<OpaqueBlock>
		+ HeaderBackend<OpaqueBlock>
		+ HeaderMetadata<OpaqueBlock, Error = BlockChainError>
		+ 'static,
	C::Api: sp_block_builder::BlockBuilder<OpaqueBlock>,
	C::Api:
		substrate_frame_rpc_system::AccountNonceApi<OpaqueBlock, AccountId, Nonce>,
	P: TransactionPool + 'static,
{
	use substrate_frame_rpc_system::{System, SystemApiServer};
	let mut module = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		deny_unsafe,
	} = deps;

	module
		.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;

	Ok(module)
}
