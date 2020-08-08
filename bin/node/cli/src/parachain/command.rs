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
use log::info;
use node_primitives::Block;
use polkadot_parachain::primitives::AccountIdConversion;
use polkadot_primitives::v0::Id as ParaId;
use sc_cli::{
    ChainSpec, CliConfiguration, ImportParams, KeystoreParams, NetworkParams, Result,
    RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_network::config::TransportConfig;
use sc_service::{
    config::{Configuration, NetworkConfiguration, NodeKeyConfig, PrometheusConfig},
    TaskManager,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
use sp_runtime::BuildStorage;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

fn generate_genesis_state() -> sc_service::error::Result<Block> {
    let storage = (&crate::parachain::chain_spec::robonomics_parachain_config()).build_storage()?;

    let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
        let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
            child_content.data.clone().into_iter().collect(),
        );
        (sk.clone(), state_root.encode())
    });
    let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
        storage.top.clone().into_iter().chain(child_roots).collect(),
    );

    let extrinsics_root =
        <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

    Ok(Block::new(
        <<Block as BlockT>::Header as HeaderT>::new(
            Zero::zero(),
            extrinsics_root,
            state_root,
            Default::default(),
            Default::default(),
        ),
        Default::default(),
    ))
}

/// Run a collator node with the given parachain `Configuration`
pub fn run(
    config: Configuration,
    parachain_id: u32,
    relaychain_args: &Vec<String>,
    validator: bool,
) -> sc_service::error::Result<TaskManager> {
    let key = Arc::new(sp_core::Pair::generate().0);
    let parachain_id = ParaId::from(parachain_id);

    let block = generate_genesis_state()?;
    let header_hex = format!("0x{:?}", HexDisplay::from(&block.header().encode()));
    let parachain_account =
        AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&parachain_id);

    info!("[Para] ID: {}", parachain_id);
    info!("[Para] Account: {}", parachain_account);
    info!("[Para] Genesis State: {}", header_hex);

    let mut polkadot_cli = PolkadotCli::from_iter(
        [PolkadotCli::executable_name().to_string()]
            .iter()
            .chain(relaychain_args.iter()),
    );
    polkadot_cli.base_path = config.base_path.as_ref().map(|x| x.path().join("polkadot"));

    let task_executor = config.task_executor.clone();
    let polkadot_config =
        SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor).unwrap();

    super::collator::run_node(config, parachain_id, key, polkadot_config, validator)
}

#[derive(Debug, structopt::StructOpt)]
pub struct PolkadotCli {
    #[structopt(flatten)]
    pub base: polkadot_cli::RunCmd,

    #[structopt(skip)]
    pub base_path: Option<std::path::PathBuf>,
}

impl SubstrateCli for PolkadotCli {
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
        2020
    }

    fn executable_name() -> String {
        "robonomics".into()
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        let chain_spec = match id {
            "" => polkadot_service::WestendChainSpec::from_json_bytes(
                &include_bytes!("../../res/polkadot_chainspec.json")[..],
            )?,
            path => {
                polkadot_service::WestendChainSpec::from_json_file(std::path::PathBuf::from(path))?
            }
        };
        Ok(Box::new(chain_spec))
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

impl CliConfiguration for PolkadotCli {
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

    fn base_path(&self) -> Result<Option<sc_service::config::BasePath>> {
        Ok(self
            .shared_params()
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self) -> Result<Option<SocketAddr>> {
        let rpc_port = self.base.base.rpc_port;
        Ok(Some(parse_address(
            &format!("127.0.0.1:{}", 9934),
            rpc_port,
        )?))
    }

    fn rpc_ws(&self) -> Result<Option<SocketAddr>> {
        let ws_port = self.base.base.ws_port;
        Ok(Some(parse_address(
            &format!("127.0.0.1:{}", 9945),
            ws_port,
        )?))
    }

    fn prometheus_config(&self) -> Result<Option<PrometheusConfig>> {
        Ok(None)
    }

    // TODO: we disable mdns for the polkadot node because it prevents the process to exit
    //       properly. See https://github.com/paritytech/cumulus/issues/57
    fn network_config(
        &self,
        chain_spec: &Box<dyn sc_service::ChainSpec>,
        is_dev: bool,
        net_config_dir: PathBuf,
        client_id: &str,
        node_name: &str,
        node_key: NodeKeyConfig,
    ) -> Result<NetworkConfiguration> {
        let (mut network, allow_private_ipv4) = self
            .network_params()
            .map(|x| {
                (
                    x.network_config(
                        chain_spec,
                        is_dev,
                        Some(net_config_dir),
                        client_id,
                        node_name,
                        node_key,
                    ),
                    !x.no_private_ipv4,
                )
            })
            .expect("NetworkParams is always available on RunCmd; qed");

        network.transport = TransportConfig::Normal {
            enable_mdns: false,
            allow_private_ipv4,
            wasm_external_transport: None,
            use_yamux_flow_control: false,
        };

        Ok(network)
    }

    fn init<C: SubstrateCli>(&self) -> Result<()> {
        unreachable!("PolkadotCli is never initialized; qed");
    }
}

// copied directly from substrate
fn parse_address(address: &str, port: Option<u16>) -> std::result::Result<SocketAddr, String> {
    let mut address: SocketAddr = address
        .parse()
        .map_err(|_| format!("Invalid address: {}", address))?;
    if let Some(port) = port {
        address.set_port(port);
    }

    Ok(address)
}
