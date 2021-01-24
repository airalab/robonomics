///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life>
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

use codec::Encode;
use cumulus_primitives::{genesis::generate_genesis_block, ParaId};
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
    config::{BasePath, Configuration, PrometheusConfig},
    TaskManager,
};
use sp_api::BlockT;
use sp_core::hexdisplay::HexDisplay;
use std::net::SocketAddr;

/// Run a collator node with the given parachain `Configuration`
pub async fn run(
    config: Configuration,
    relaychain_args: &Vec<String>,
    parachain_id: Option<u32>,
    validator: bool,
) -> sc_service::error::Result<TaskManager> {
    let key = sp_core::Pair::generate().0;

    let extension = super::chain_spec::Extensions::try_get(&config.chain_spec);
    let parachain_id = ParaId::from(parachain_id.or(extension.map(|e| e.para_id)).unwrap_or(100));
    let relay_chain_id = extension.map(|e| e.relay_chain.clone());
    let polkadot_cli = RelayChainCli::new(
        config.base_path.as_ref().map(|x| x.path().join("polkadot")),
        relay_chain_id,
        [RelayChainCli::executable_name().to_string()]
            .iter()
            .chain(relaychain_args.iter()),
    );

    let block: node_primitives::Block =
        generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
    let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));
    let parachain_account =
        AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&parachain_id);

    info!("[Parachain] ID: {}", parachain_id);
    info!("[Parachain] Account: {}", parachain_account);
    info!("[Parachain] Genesis State: {}", genesis_state);
    info!(
        "[Parachain] Is collating: {}",
        if validator { "yes" } else { "no" }
    );

    let task_executor = config.task_executor.clone();
    let polkadot_config =
        SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor, None)
            .map_err(|err| format!("Relay chain argument error: {}", err))?;

    super::collator::start_node(config, key, polkadot_config, parachain_id, validator)
        .await
        .map(|r| r.0)
}

#[derive(Debug)]
pub struct RelayChainCli {
    /// The actual relay chain cli object.
    pub base: polkadot_cli::RunCmd,

    /// Optional chain id that should be passed to the relay chain.
    pub chain_id: Option<String>,

    /// The base path that should be used by the relay chain.
    pub base_path: Option<std::path::PathBuf>,
}

impl RelayChainCli {
    /// Create a new instance of `Self`.
    pub fn new<'a>(
        base_path: Option<std::path::PathBuf>,
        chain_id: Option<String>,
        relay_chain_args: impl Iterator<Item = &'a String>,
    ) -> Self {
        use structopt::StructOpt;

        Self {
            base_path,
            chain_id,
            base: polkadot_cli::RunCmd::from_iter(relay_chain_args),
        }
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Robonomics Network Parachain Collator".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Robonomics parachain collator\n\nThe command-line arguments provided first will be \
        passed to the parachain node, while the arguments provided after -- will be passed \
        to the relaychain node.\n\n\
        {} [parachain-args] -- [relaychain-args]",
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
        2017
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        if id == "rococo_local_testnet" {
            Ok(Box::new(
                polkadot_service::RococoChainSpec::from_json_bytes(
                    &include_bytes!("../../res/rococo_local_testnet.json")[..],
                )
                .unwrap(),
            ))
        } else {
            polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter())
                .load_spec(id)
        }
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_ws_listen_port() -> u16 {
        9945
    }

    fn rpc_http_listen_port() -> u16 {
        9934
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
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> Result<Option<String>> {
        self.base.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_ws(default_listen_port)
    }

    fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port)
    }

    fn init<C: SubstrateCli>(&self) -> Result<sc_telemetry::TelemetryWorker> {
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

    fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool()
    }

    fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
        self.base.base.state_cache_child_ratio()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
        self.base.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
        self.base.base.telemetry_external_transport()
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
}
