//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::{sync::Arc, time::Duration};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use template_node_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use substrate_service::{
	FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
	FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
	TaskExecutor,
};
use node_executor;
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration, NothingExtra};
use client;
use grandpa;
use primitives::ed25519::Pair;
use runtime_primitives::BasicInherentData as InherentData;

pub use substrate_executor::NativeExecutor;
// Our native executor instance.
native_executor_instance!(
	pub Executor,
	template_node_runtime::api::dispatch,
	template_node_runtime::native_version,
	include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/template_node_runtime.compact.wasm")
);

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
	pub grandpa_import_setup: Option<(Arc<grandpa::BlockImportForService<F>>, grandpa::LinkHalfForService<F>)>,
}

impl<F> Default for NodeConfig<F> where F: substrate_service::ServiceFactory {
	fn default() -> NodeConfig<F> {
		NodeConfig {
			grandpa_authority: false,
			grandpa_authority_only: false,
			grandpa_import_setup: None,
		}
	}
}

construct_service_factory! {
	struct Factory {
		Block = Block,
		RuntimeApi = RuntimeApi,
		NetworkProtocol = NodeProtocol { |config| Ok(NodeProtocol::new()) },
		RuntimeDispatch = node_executor::Executor,
		FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>, Block>
			{ |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
		Genesis = GenesisConfig,
		Configuration = NodeConfig<Self>,
		FullService = FullComponents<Self>
			{ |config: FactoryFullConfiguration<Self>, executor: TaskExecutor|
				FullComponents::<Factory>::new(config, executor) },
		AuthoritySetup = {
			|mut service: Self::FullService, executor: TaskExecutor, key: Option<Arc<Pair>>| {
				let (block_import, link_half) = service.config.custom.grandpa_import_setup.take()
					.expect("Link Half and Block Import are present for Full Services or setup failed before. qed");

				let local_key = if let Some(key) = key {
					if !service.config.custom.grandpa_authority_only {
						info!("Using authority key {}", key.public());
						let proposer = Arc::new(substrate_service::ProposerFactory {
							client: service.client(),
							transaction_pool: service.transaction_pool(),
						});

						let client = service.client();
						executor.spawn(start_aura(
							SlotDuration::get_or_compute(&*client)?,
							key.clone(),
							client,
							block_import.clone(),
							proposer,
							service.network(),
						));
					}

					if service.config.custom.grandpa_authority {
						info!("Running Grandpa session as Authority {}", key.public());
						Some(key)
					} else {
						None
					}
				} else {
					None
				};

				let voter = grandpa::run_grandpa(
					grandpa::Config {
						local_key,
						gossip_duration: Duration::new(4, 0), // FIXME: make this available through chainspec?
						name: Some(service.config.name.clone())
					},
					link_half,
					grandpa::NetworkBridge::new(service.network()),
				)?;

				executor.spawn(voter);

				Ok(service)
			}
		},
		LightService = LightComponents<Self>
			{ |config, executor| <LightComponents<Factory>>::new(config, executor) },
		FullImportQueue = AuraImportQueue<
			Self::Block,
			grandpa::BlockImportForService<Self>,
			NothingExtra,
			::consensus::InherentProducingFn<InherentData>,
		>
			{ |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>| {
				let slot_duration = SlotDuration::get_or_compute(&*client)?;
				let (block_import, link_half) = grandpa::block_import::<_, _, _, RuntimeApi, FullClient<Self>>(client.clone(), client)?;
				let block_import = Arc::new(block_import);

				config.custom.grandpa_import_setup = Some((block_import.clone(), link_half));

				Ok(import_queue(
					slot_duration,
					block_import,
					NothingExtra,
					::consensus::make_basic_inherent as _,
				))
			}},
		LightImportQueue = AuraImportQueue<
			Self::Block,
			LightClient<Self>,
			NothingExtra,
			::consensus::InherentProducingFn<InherentData>,
		>
			{ |ref mut config, client: Arc<LightClient<Self>>|
				Ok(import_queue(
					SlotDuration::get_or_compute(&*client)?,
					client,
					NothingExtra,
					::consensus::make_basic_inherent as _,
				))
			},
	}
}
