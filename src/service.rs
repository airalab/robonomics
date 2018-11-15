//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::sync::Arc;
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use template_node_runtime::{self, GenesisConfig, opaque::Block, ClientWithApi};
use substrate_service::{
	FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
	FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
	Roles, TaskExecutor,
};
use consensus::{import_queue, start_aura, Config as AuraConfig, AuraImportQueue};
use client;

pub use substrate_executor::NativeExecutor;
// Our native executor instance.
native_executor_instance!(
	pub Executor,
	template_node_runtime::api::dispatch,
	template_node_runtime::native_version,
	include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/template_node_runtime.compact.wasm")
);

const AURA_SLOT_DURATION: u64 = 6;

construct_simple_protocol! {
	/// Demo protocol attachment for substrate.
	pub struct NodeProtocol where Block = Block { }
}

construct_service_factory! {
	struct Factory {
		Block = Block,
		RuntimeApi = ClientWithApi,
		NetworkProtocol = NodeProtocol { |config| Ok(NodeProtocol::new()) },
		RuntimeDispatch = Executor,
		FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, ClientWithApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, ClientWithApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		Genesis = GenesisConfig,
		Configuration = (),
		FullService = FullComponents<Self>
			{ |config: FactoryFullConfiguration<Self>, executor: TaskExecutor| {
				let is_auth = config.roles == Roles::AUTHORITY;
				FullComponents::<Factory>::new(config, executor.clone()).map(move |service|{
					if is_auth {
						if let Ok(Some(Ok(key))) = service.keystore().contents()
							.map(|keys| keys.get(0).map(|k| service.keystore().load(k, "")))
						{
							info!("Using authority key {}", key.public());
							let task = start_aura(
								AuraConfig {
									local_key:  Some(Arc::new(key)),
									slot_duration: AURA_SLOT_DURATION,
								},
								service.client(),
								service.proposer(),
								service.network(),
							);

							executor.spawn(task);
						}
					}

					service
				})
			}
		},
		LightService = LightComponents<Self>
			{ |config, executor| <LightComponents<Factory>>::new(config, executor) },
		FullImportQueue = AuraImportQueue<Self::Block, FullClient<Self>>
			{ |config, client| Ok(import_queue(AuraConfig {
						local_key: None,
						slot_duration: 5
					}, client)) },
		LightImportQueue = AuraImportQueue<Self::Block, LightClient<Self>>
			{ |config, client| Ok(import_queue(AuraConfig {
						local_key: None,
						slot_duration: 5
					}, client)) },
	}
}
