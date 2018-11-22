//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::{sync::Arc, time::Duration};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use template_node_runtime::{self, GenesisConfig, opaque::Block, ClientWithApi};
use substrate_service::{
	FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
	FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
	TaskExecutor,
};
use consensus::{import_queue, start_aura, Config as AuraConfig, AuraImportQueue, NothingExtra};
use client;
use grandpa;
use primitives::ed25519::Pair;

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

/// Node specific configuration
pub struct NodeConfig<F: substrate_service::ServiceFactory> {
	/// should run as a grandpa authority
	pub grandpa_authority: bool,
	/// should run as a grandpa authority only, don't validate as usual
	pub grandpa_authority_only: bool,
	/// grandpa connection to import block

	// FIXME: rather than putting this on the config, let's have an actual intermediate setup state
	// https://github.com/paritytech/substrate/issues/1134
	pub grandpa_link_half: Option<grandpa::LinkHalfForService<F>>,
}

impl<F> Default for NodeConfig<F> where F: substrate_service::ServiceFactory {
	fn default() -> NodeConfig<F> {
		NodeConfig {
			grandpa_authority: false,
			grandpa_authority_only: false,
			grandpa_link_half: None
		}
	}
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
		Configuration = NodeConfig<Self>,
		FullService = FullComponents<Self>
			{ |config: FactoryFullConfiguration<Self>, executor: TaskExecutor|
				FullComponents::<Factory>::new(config, executor) },
		AuthoritySetup = {
			|service: Self::FullService, executor: TaskExecutor, key: Arc<Pair>| {
				if service.config.custom.grandpa_authority {
					info!("Running Grandpa session as Authority {}", key.public());
					let link_half = service.config().custom.grandpa_link_half.as_ref().take()
						.expect("Link Half is present for Full Services or setup failed before. qed");
					let grandpa_fut = grandpa::run_grandpa(
						grandpa::Config {
							gossip_duration: Duration::new(4, 0), // FIXME: make this available through chainspec?
							local_key: Some(key.clone()),
							name: Some(service.config().name.clone())
						},
						(*link_half).clone(),
						grandpa::NetworkBridge::new(service.network())
					)?;

					executor.spawn(grandpa_fut);
				}
				if !service.config.custom.grandpa_authority_only {
					info!("Using authority key {}", key.public());
					executor.spawn(start_aura(
						AuraConfig {
							local_key: Some(key),
							slot_duration: AURA_SLOT_DURATION,
						},
						service.client(),
						service.proposer(),
						service.network(),
					));
				}
				Ok(service)
			}
		},
		LightService = LightComponents<Self>
			{ |config, executor| <LightComponents<Factory>>::new(config, executor) },
		FullImportQueue = AuraImportQueue<Self::Block, FullClient<Self>, NothingExtra>
			{ |config, client| Ok(import_queue(AuraConfig {
						local_key: None,
						slot_duration: 5
					},
					client,
					NothingExtra
			)) },
		LightImportQueue = AuraImportQueue<Self::Block, LightClient<Self>, NothingExtra>
			{ |config, client| Ok(import_queue(AuraConfig {
						local_key: None,
						slot_duration: 5
					},
					client,
					NothingExtra
			)) },
	}
}
