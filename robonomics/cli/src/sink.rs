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

use crate::error::Result;
use robonomics_protocol::pubsub::Multiaddr;
use robonomics_io::source::virt::Stdin;
use robonomics_io::sink::virt;
use robonomics_io::Consumer;
use std::time::Duration;
use futures::StreamExt;
use async_std::task;

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
        /// Substrate node WebSocket endpoint
        #[structopt(long, default_value = "ws://localhost:9944")]
        remote: String,
        /// Sender account seed URI
        #[structopt(short)]
        suri: String,
    },
}

impl SinkCmd {
    pub fn run(&self) -> Result<()> {
        let stdin = Stdin::new().boxed();
        match self.clone() {
            SinkCmd::PubSub { topic_name, listen, bootnodes, hearbeat_secs } => {
                let hearbeat = Duration::from_secs(hearbeat_secs);
                let device = virt::PubSub::new(listen, bootnodes, topic_name, hearbeat).unwrap();
                task::block_on(device.consume(stdin))
            }
            SinkCmd::Datalog { remote, suri } => {
                let device = virt::Datalog::new(remote, suri).unwrap();
                let bytestream = stdin.map(|s| Vec::from(s.as_bytes())).boxed();
                task::block_on(device.consume(bytestream))
            }
        }
        Ok(())
    }
}
