//! Service and ServiceFactory implementation. Specialized wrapper over Substrate service.

#![warn(unused_extern_crates)]

use std::sync::Arc;
use transaction_pool::{self, txpool::{Pool as TransactionPool}};
use robonomics_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use substrate_service::{
    FactoryFullConfiguration, LightComponents, FullComponents, FullBackend,
    FullClient, LightClient, LightBackend, FullExecutor, LightExecutor,
    TaskExecutor,
};
use consensus::{import_queue, start_aura, AuraImportQueue, SlotDuration, NothingExtra};
use client;
use primitives::ed25519::Pair;
use runtime_primitives::BasicInherentData as InherentData;
use basic_authorship::ProposerFactory;
use ros_integration::start_ros;

pub use substrate_executor::NativeExecutor;
/// Robonomics native executor instance.
native_executor_instance!(
    pub Executor,
    robonomics_runtime::api::dispatch,
    robonomics_runtime::native_version,
    include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/robonomics_runtime.compact.wasm")
);

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
        Configuration = (),
        FullService = FullComponents<Self> {
            |config: FactoryFullConfiguration<Self>, executor: TaskExecutor| {
                let service = FullComponents::<Factory>::new(config, executor.clone()).unwrap();

                executor.spawn(start_ros(
                    service.network(),
                    service.client(),
                    service.on_exit()
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
                    ));
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
            ::consensus::InherentProducingFn<InherentData>,
        >
            { |config: &mut FactoryFullConfiguration<Self> , client: Arc<FullClient<Self>>|
                Ok(import_queue(
                    SlotDuration::get_or_compute(&*client)?,
                    client,
                    NothingExtra,
                    ::consensus::make_basic_inherent as _,
                ))
            },
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
