///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
//! Console line interface command implementation.

use crate::{
    chain_spec::{self, RobonomicsChain, RobonomicsFamily},
    cli::{Cli, RelayChainCli, Subcommand},
};
use robonomics_primitives::{AccountId, Block, CommunityAccount};
use robonomics_service as service;

use cumulus_client_service::storage_proof_size::HostFunctions as ReclaimHostFunctions;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::{info, warn};
use sc_chain_spec::ChainSpec;
use sc_cli::{
    CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams,
    Result, SharedParams, SubstrateCli,
};
use sc_service::{config::PrometheusConfig, BasePath};
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{AccountIdConversion, IdentifyAccount};

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "airalab-robonomics".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/airalab/robonomics/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::dev::config()),
            "kusama" => Box::new(chain_spec::mainnet::kusama_config()),
            "polkadot" => Box::new(chain_spec::mainnet::polkadot_config()),
            path => Box::new(chain_spec::dev::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Airalab Robonomics".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Airalab Robonomics\n\nThe command-line arguments provided first will be \
        passed to the parachain node, while the arguments provided after -- will be passed \
        to the relay chain node.\n\n\
        {} [parachain-args] -- [relay_chain-args]",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/airalab/robonomics/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
    }
}

/// Creates partial components for the runtimes that are supported by the benchmarks.
macro_rules! construct_benchmark_partials {
    ($config:expr, |$partials:ident| $code:expr) => {
        match $config.chain_spec.family() {
            RobonomicsFamily::Development => {
                let $partials = service::dev::new_partial::<dev_runtime::RuntimeApi>(&$config)?;
                $code
            }
            _ => Err("The chain is not supported".into()),
        }
    };
}

macro_rules! construct_async_run {
    (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
        let runner = $cli.create_runner($cmd)?;
        match runner.config().chain_spec.family() {
            RobonomicsFamily::Development => {
                runner.async_run(|$config| {
                    let $components = service::dev::new_partial::<dev_runtime::RuntimeApi>(
                        &$config,
                    )?;
                    let task_manager = $components.task_manager;
                    { $( $code )* }.map(|v| (v, task_manager))
                })
            },
            _ => Err("The chain is not supported".into()),
        }
    }}
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;

            if cli.run.base.shared_params.is_dev() {
                // Just create dev service in dev mode
                runner.run_node_until_exit(|config| async move {
                    service::dev::new_service::<dev_runtime::RuntimeApi>(config)
                        .map_err(sc_cli::Error::Service)
                        .map(|(task_manager, _, _, _)| task_manager)
                })
            } else {
                // Else it's collator, let's run it
                let collator_options = cli.run.collator_options();

                let treasury_account = CommunityAccount::Treasury.into_account();
                let lighthouse_account = if let Some(account) = cli.lighthouse_account {
                    AccountId::from_ss58check(account.as_str()).unwrap_or(treasury_account)
                } else {
                    // Treasury by default
                    treasury_account
                };

                runner.run_node_until_exit(|config| async move {
                    let hwbench = (!cli.no_hardware_benchmarks)
                        .then_some(config.database.path().map(|database_path| {
                            let _ = std::fs::create_dir_all(database_path);
                            sc_sysinfo::gather_hwbench(
                                Some(database_path),
                                &SUBSTRATE_REFERENCE_HARDWARE,
                            )
                        }))
                        .flatten();

                    let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
                        .map(|e| e.para_id)
                        .ok_or("Could not find parachain extension in chain-spec.")?;

                    let polkadot_cli = RelayChainCli::new(
                        &config,
                        [RelayChainCli::executable_name()]
                            .iter()
                            .chain(cli.relaychain_args.iter()),
                    );

                    let id = ParaId::from(para_id);
                    let parachain_account =
                        AccountIdConversion::<AccountId>::into_account_truncating(&id);
                    let tokio_handle = config.tokio_handle.clone();
                    let polkadot_config = SubstrateCli::create_configuration(
                        &polkadot_cli,
                        &polkadot_cli,
                        tokio_handle,
                    )
                    .map_err(|err| format!("Relay chain argument error: {}", err))?;

                    info!("Parachain id: {:?}", id);
                    info!("Parachain Account: {}", parachain_account);
                    if config.role.is_authority() {
                        info!("Is lighthouse: {}", lighthouse_account);
                    }

                    match config.chain_spec.family() {
                        RobonomicsFamily::Mainnet => {
                            service::parachain::start_generic_robonomics_parachain::<
                                generic_runtime::RuntimeApi,
                            >(
                                config,
                                polkadot_config,
                                collator_options,
                                id,
                                lighthouse_account,
                                hwbench,
                            )
                            .await
                            .map_err(sc_cli::Error::Service)
                        }
                        _ => panic!("not implemented"),
                    }
                })
            }
        }
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::Sign(cmd)) => cmd.run(),
        Some(Subcommand::Verify(cmd)) => cmd.run(),
        Some(Subcommand::Vanity(cmd)) => cmd.run(),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.database))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.chain_spec))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::ExportGenesisState(cmd)) => {
            // construct_async_run!(|components, cli, cmd, config| {
            //     Ok(async move { cmd.run(&*config.chain_spec, &*components.client) })
            // })
            Ok(())
        }
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            // Switch on the concrete benchmark sub-command-
            match cmd {
                BenchmarkCmd::Pallet(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
                        // runner.sync_run(|config| cmd.run::<Block, ()>(config))
                        runner.sync_run(|config| cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ReclaimHostFunctions>(Some(config.chain_spec)))
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`."
                            .into())
                    }
                }
                BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
                    construct_benchmark_partials!(config, |partials| cmd.run(partials.client))
                }),
                #[cfg(not(feature = "runtime-benchmarks"))]
                BenchmarkCmd::Storage(_) => {
                    return Err(sc_cli::Error::Input(
                        "Compile with --features=runtime-benchmarks \
                        to enable storage benchmarks."
                            .into(),
                    )
                    .into())
                }
                #[cfg(feature = "runtime-benchmarks")]
                BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
                    construct_benchmark_partials!(config, |partials| {
                        let db = partials.backend.expose_db();
                        let storage = partials.backend.expose_storage();

                        cmd.run(config, partials.client.clone(), db, storage)
                    })
                }),
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                }
                // NOTE: this allows the Client to leniently implement
                // new benchmark commands without requiring a companion MR.
                #[allow(unreachable_patterns)]
                _ => Err("Benchmarking sub-command unsupported".into()),
            }
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_listen_port() -> u16 {
        9945
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()?
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_addr(
        &self,
        default_listen_port: u16,
    ) -> Result<std::option::Option<Vec<sc_cli::RpcEndpoint>>> {
        self.base.base.rpc_addr(default_listen_port)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base
            .base
            .prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(&self, _support_url: &String, _impl_version: &String, _logger_hook: F) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder),
    {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
    }

    fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
        self.base.base.trie_cache_maximum_size()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_max_connections(&self) -> Result<u32> {
        self.base.base.rpc_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }

    fn node_name(&self) -> Result<String> {
        self.base.base.node_name()
    }
}
