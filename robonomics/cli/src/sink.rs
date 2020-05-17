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
//! Robonomics data sink interface.

#![deny(missing_docs)]

use crate::error::Result;
use robonomics_protocol::pubsub::Multiaddr;
use robonomics_io::stream::virt::stdin;
use robonomics_io::sink::virt;
use std::time::Duration;
use futures::prelude::*;
use async_std::task;

/// Sink device commands.
#[derive(structopt::StructOpt, Clone, Debug)]
pub enum SinkCmd {
    /// Broadcast data into PubSub topic.
    #[structopt(name = "pubsub")]
    PubSub {
        /// Publish data into given topic name.
        topic_name: String,
        /// Listen address for incoming connections. 
        #[structopt(
            long,
            value_name = "MULTIADDR",
            default_value = "/ip4/0.0.0.0/tcp/0",
        )]
        listen: Multiaddr,
        /// Indicates PubSub nodes for first connections.
        #[structopt(
            long,
            value_name = "MULTIADDR",
            use_delimiter = true,
        )]
        bootnodes: Vec<Multiaddr>,
        /// How often node should check another nodes availability, in secs.
        #[structopt(
            long,
            value_name = "HEARTBEAT_SECS",
            default_value = "5",
        )]
        hearbeat_secs: u64
    },
    /// Data blockchainization subsystem command.
    Datalog {
        /// Substrate node WebSocket endpoint.
        #[structopt(long, default_value = "ws://localhost:9944")]
        remote: String,
        /// Sender account seed URI.
        #[structopt(short)]
        suri: String,
    },
    /// Upload data into IPFS storage.
    Ipfs {
        /// IPFS node API endpoint.
        #[structopt(long, default_value = "https://127.0.0.1:5001")]
        remote: String,
    }
}

impl SinkCmd {
    /// Write data into sink device.
    pub fn run(&self) -> Result<()> {
        match self.clone() {
            SinkCmd::PubSub { topic_name, listen, bootnodes, hearbeat_secs } => {
                let hearbeat = Duration::from_secs(hearbeat_secs);
                let pubsub = virt::pubsub(listen, bootnodes, topic_name, hearbeat)?;
                task::block_on(stdin().forward(pubsub))?;
            }
            SinkCmd::Datalog { remote, suri } => {
                let (submit, hashes) = virt::datalog(remote, suri)?;
                task::spawn(stdin().forward(submit));
                let hex_encoded = hashes.map(|r| r.map(|h| hex::encode(h)));
                task::block_on(hex_encoded.forward(virt::stdout()))?;
            }
            SinkCmd::Ipfs { remote } => {
                let (upload, hashes) = virt::ipfs(remote.as_str())?;
                task::spawn(stdin().forward(upload));
                task::block_on(hashes.forward(virt::stdout()))?;
            }
        }
        Ok(())
    }
}
