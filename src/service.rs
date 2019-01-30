//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use substrate_service::{
    FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
    FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
    TaskExecutor,
};
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration, NothingExtra};
use robonomics_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use basic_authorship::ProposerFactory;
use inherents::InherentDataProviders;
use primitives::ed25519::Pair;
use std::sync::Arc;
use client;

pub use substrate_executor::NativeExecutor;
/// Robonomics runtime native executor instance.
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm")
);

#[derive(Default)]
pub struct NodeConfig {
    inherent_data_providers: InherentDataProviders,
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
        Configuration = NodeConfig,
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
            |service: Self::FullService, executor: TaskExecutor, key: Option<Arc<Pair>>| {
                if let Some(key) = key {
                    info!("Using authority key {}", key.public());
                    let client = service.client();
                    let proposer = Arc::new(ProposerFactory {
                        client: client.clone(),
                        transaction_pool: service.transaction_pool(),
                    });
                    executor.spawn(start_aura(
                        SlotDuration::get_or_compute(&*client)?,
                        key.clone(),
                        client.clone(),
                        client,
                        proposer,
                        service.network(),
                        service.on_exit(),
                        service.config.custom.inherent_data_providers.clone(),
                    )?);
                }

                Ok(service)
            }
        },
        LightService = LightComponents<Self>
            { |config, executor| <LightComponents<Factory>>::new(config, executor) },
        FullImportQueue = AuraImportQueue<
            Self::Block,
            FullClient<Self>,
            NothingExtra,
        >
            { |config: &mut FactoryFullConfiguration<Self>, client: Arc<FullClient<Self>>|
                import_queue(
                    SlotDuration::get_or_compute(&*client)?,
                    client.clone(),
                    None,
                    client,
                    NothingExtra,
                    config.custom.inherent_data_providers.clone(),
                ).map_err(Into::into)
            },
        LightImportQueue = AuraImportQueue<
            Self::Block,
            LightClient<Self>,
            NothingExtra,
        >
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
