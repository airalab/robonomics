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

use async_std::task;
use libp2p::Multiaddr;
use crate::pubsub::*;
use crate::error::Result;

/// The PubSub command for pubsub router mode.
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct PubSubCmd {
    /// Topic name for subscribe and publish.
    #[structopt(
        long,
        value_name = "TOPIC_NAME",
    )]
    pub topic: Option<String>,
    /// Listen address for incoming PubSub connections,
    #[structopt(
        long,
        value_name = "MULTIADDR",
        default_value = "/ip4/0.0.0.0/tcp/0",
    )]
    pub listen: Multiaddr,
    /// Indicates PubSub nodes for first connections
    #[structopt(
        long,
        value_name = "MULTIADDR",
        use_delimiter = true,
    )]
    pub bootnodes: Vec<Multiaddr>,
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
        let mut pubsub = Gossipsub::new()?;

        // Listen address
        pubsub.listen(&self.listen)?;

        // Connect to bootnodes
        for addr in &self.bootnodes {
            pubsub.connect(addr)?;
        }

        // Subscribe on topic topic and print received content
        match self.topic.clone() {
            Some(topic_name) => {
                pubsub.subscribe(topic_name, |_, msg|
                    log::info!(
                        target: "robonomics-pubsub",
                        "RECEIVED: {}", String::from_utf8_lossy(&msg)
                    )
                );
            },
            _ => (),
        }

        Ok(task::block_on(pubsub.start()))
    }
}
