//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::time::Duration;

use substrate_service::{
    FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
    FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
    TaskExecutor,
};
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration, NothingExtra};
use robonomics_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use inherents::InherentDataProviders;
use primitives::ed25519::Pair;
use std::sync::Arc;
use grandpa;
use client;

pub use substrate_executor::NativeExecutor;
/// Robonomics runtime native executor instance.
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm")
);

pub struct NodeConfig<F: substrate_service::ServiceFactory> {
    inherent_data_providers: InherentDataProviders,
    pub grandpa_import_setup: Option<(Arc<grandpa::BlockImportForService<F>>, grandpa::LinkHalfForService<F>)>,
}

impl<F> Default for NodeConfig<F> where F: substrate_service::ServiceFactory {
    fn default() -> NodeConfig<F> {
        NodeConfig {
            grandpa_import_setup: None,
            inherent_data_providers: InherentDataProviders::new(),
        }
    }
}

construct_simple_protocol! {
    /// Robonomics protocol attachment for substrate.
    pub struct Protocol where Block = Block { }
}

construct_service_factory! {
    struct Factory {
        Block = Block,
        RuntimeApi = RuntimeApi,
        NetworkProtocol = Protocol { |config| Ok(Protocol::new()) },
        RuntimeDispatch = Executor,
        FullTransactionPoolApi = transaction_pool::ChainApi<client::Client<FullBackend<Self>, FullExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        LightTransactionPoolApi = transaction_pool::ChainApi<client::Client<LightBackend<Self>, LightExecutor<Self>, Block, RuntimeApi>, Block>
            { |config, client| Ok(TransactionPool::new(config, transaction_pool::ChainApi::new(client))) },
        Genesis = GenesisConfig,
        Configuration = NodeConfig<Self>,
        FullService = FullComponents<Self> {
            |config: FactoryFullConfiguration<Self>, executor: TaskExecutor| {
                let service = FullComponents::<Factory>::new(config, executor.clone()).unwrap();

                #[cfg(feature = "ros")]
                executor.spawn(ros_integration::start_ros(
                    service.network(),
                    service.client(),
                    service.transaction_pool(),
                    service.keystore(),
                    service.on_exit(),
                ));

                Ok(service)
            }
        },
        AuthoritySetup = {
            |mut service: Self::FullService, executor: TaskExecutor, local_key: Option<Arc<Pair>>| {
                let (block_import, link_half) = service.config.custom.grandpa_import_setup.take()
                    .expect("Link Half and Block Import are present for Full Services or setup failed before. qed");
                if let Some(ref key) = local_key {
                    info!("Using authority key {}", key.public());
                    let proposer = Arc::new(basic_authorship::ProposerFactory {
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
                        service.on_exit(),
                        service.config.custom.inherent_data_providers.clone(),
                    )?);

                    info!("Running Grandpa session as Authority {}", key.public());
                }

                executor.spawn(grandpa::run_grandpa(
                    grandpa::Config {
                        local_key,
                        gossip_duration: Duration::new(4, 0),
                        justification_period: 4096,
                        name: Some(service.config.name.clone())
                    },
                    link_half,
                    grandpa::NetworkBridge::new(service.network()),
                    service.config.custom.inherent_data_providers.clone(),
                    service.on_exit(),
                )?);

                Ok(service)
            }
        },
        LightService = LightComponents<Self>
            { |config, executor| <LightComponents<Factory>>::new(config, executor) },
        FullImportQueue = AuraImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>| {
                let slot_duration = SlotDuration::get_or_compute(&*client)?;
                let (block_import, link_half) =
                    grandpa::block_import::<_, _, _, RuntimeApi, FullClient<Self>>(
                        client.clone(), client.clone()
                    )?;
                let block_import = Arc::new(block_import);
                let justification_import = block_import.clone();

                config.custom.grandpa_import_setup = Some((block_import.clone(), link_half));

                import_queue(
                    slot_duration,
                    block_import,
                    Some(justification_import),
                    client,
                    NothingExtra,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            }},
        LightImportQueue = AuraImportQueue<Self::Block>
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<LightClient<Self>>|
                import_queue(
                    SlotDuration::get_or_compute(&*client)?,
                    client.clone(),
                    None,
                    client,
                    NothingExtra,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            },
    }
}
