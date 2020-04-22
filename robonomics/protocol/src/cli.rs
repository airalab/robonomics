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
//! Robonomics Network console interface.

use sp_core::{sr25519, crypto::{Pair, Ss58Codec}};
use futures::{future, StreamExt};
use libp2p::Multiaddr;
use async_std::task;
use crate::datalog;
use crate::pubsub::*;
use crate::error::Result;

/// Command for pubsub router mode.
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct PubSubCmd {
    /// Subscribe for given topic name and print received messages.
    #[structopt(
        long,
        value_name = "TOPIC_NAME",
    )]
    pub subscribe: Option<String>,
    /// Publish stdin lines into given topic name.
    #[structopt(
        long,
        value_name = "TOPIC_NAME",
        default_value = "/ip4/0.0.0.0/tcp/0",
    )]
    pub listen: Multiaddr,
    /// Indicates PubSub nodes for first connections.
    #[structopt(
        long,
        value_name = "MULTIADDR",
        use_delimiter = true,
    )]
    pub bootnodes: Vec<Multiaddr>,
    /// Disable Robonomics PubSub peer discovery. 
    #[structopt(long)]
    pub disable_discovery: bool,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub shared_params: sc_cli::SharedParams,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub import_params: sc_cli::ImportParams,
}

impl sc_cli::CliConfiguration for PubSubCmd {
    fn shared_params(&self) -> &sc_cli::SharedParams {
        &self.shared_params
    }

    fn import_params(&self) -> Option<&sc_cli::ImportParams> {
        Some(&self.import_params)
    }
}

impl PubSubCmd {
    /// Runs the command and node as pubsub router.
    pub fn run(&self) -> Result<()> {
        let (pubsub, worker) = Gossipsub::new()?;

        // Listen address
        let _ = pubsub.listen(self.listen.clone());

        // Connect to bootnodes
        for addr in &self.bootnodes {
            let _ = pubsub.connect(addr.clone());
        }

        // Subscribe on topic topic and print received content
        match self.subscribe.clone() {
            Some(topic_name) => {
                task::spawn(pubsub.subscribe(&topic_name).for_each(|msg| {
                    println!(
                        "{}: {}",
                        msg.from.to_base58(),
                        String::from_utf8_lossy(&msg.data[..]),
                    );
                    future::ready(())
                }));
            }
            _ => (),
        }

        // Enable peer discovery if not disabled
        if !self.disable_discovery {
            task::spawn(discovery::start(pubsub.clone()));
        }

        task::block_on(worker)
    }
}

/// Wrapper type for byte vector.
type Bytes = Vec<u8>;

/// Command for data blockchainization.
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct DatalogCmd {
    /// Substrate node WebSocket endpoint
    #[structopt(long, default_value = "ws://localhost:9944")]
    remote: String,
    /// Sender account seed URI
    #[structopt(short)]
    suri: String,
    /// Hex encoded data record to send (without 0x prefix)
    #[structopt(short, parse(try_from_str = hex::decode))]
    record: Bytes,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub shared_params: sc_cli::SharedParams,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub import_params: sc_cli::ImportParams,
}

impl sc_cli::CliConfiguration for DatalogCmd {
    fn shared_params(&self) -> &sc_cli::SharedParams {
        &self.shared_params
    }

    fn import_params(&self) -> Option<&sc_cli::ImportParams> {
        Some(&self.import_params)
    }
}

impl DatalogCmd {
    /// Runs the command and node as pubsub router.
    pub fn run(&self) -> Result<()> {
        let signer = sr25519::Pair::from_string(self.suri.as_str(), None)?;
        log::info!(
            target: "robonomics-datalog",
            "Key loaded: {}", signer.public().to_ss58check(),
        );

        task::block_on(datalog::submit(signer, self.remote.as_str(), self.record.clone()))
    }
}
