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
use async_std::task;
use futures::prelude::*;
use robonomics_io::sink::virt;
use robonomics_io::source::virt::stdin;
use robonomics_protocol::pubsub::Multiaddr;
use std::time::Duration;

/// Sink device commands.
#[derive(structopt::StructOpt, Clone, Debug)]
pub enum SinkCmd {
    /// Broadcast data into PubSub topic.
    #[structopt(name = "pubsub")]
    PubSub {
        /// Publish data into given topic name.
        topic_name: String,
        /// Listen address for incoming connections.
        #[structopt(long, value_name = "MULTIADDR", default_value = "/ip4/0.0.0.0/tcp/0")]
        listen: Multiaddr,
        /// Indicates PubSub nodes for first connections.
        #[structopt(long, value_name = "MULTIADDR", use_delimiter = true)]
        bootnodes: Vec<Multiaddr>,
        /// How often node should check another nodes availability, in secs.
        #[structopt(long, value_name = "HEARTBEAT_SECS", default_value = "5")]
        hearbeat_secs: u64,
    },
    /// Data blockchainization subsystem command.
    Datalog {
        /// Substrate node WebSocket endpoint.
        #[structopt(long, value_name = "REMOTE_URI", default_value = "ws://localhost:9944")]
        remote: String,
        /// Sender account seed URI.
        #[structopt(short, value_name = "SECRET_URI")]
        suri: String,
    },
    /// Upload data into IPFS storage.
    Ipfs,
    /// CPS launch subsystem command.
    Launch {
        /// Substrate node WebSocket endpoint.
        #[structopt(long, value_name = "REMOTE_URI", default_value = "ws://localhost:9944")]
        remote: String,
        /// Sender account seed URI.
        #[structopt(short, value_name = "SECRET_URI")]
        suri: String,
        /// Target CPS address.
        #[structopt(short, value_name = "ROBOT_ADDRESS")]
        robot: String,
    },
    #[cfg(feature = "ros")]
    /// Publish data into ROS topic.
    Ros {
        /// Topic name.
        #[structopt(value_name = "TOPIC_NAME")]
        topic_name: String,
        /// Queue size.
        #[structopt(long, default_value = "10")]
        queue_size: usize,
    },
}

impl SinkCmd {
    /// Write data into sink device.
    pub fn run(&self) -> Result<()> {
        match self.clone() {
            SinkCmd::PubSub {
                topic_name,
                listen,
                bootnodes,
                hearbeat_secs,
            } => {
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
            SinkCmd::Ipfs => {
                actix_rt::System::run(|| {
                    let (upload, hashes) = virt::ipfs().expect("ipfs launch");
                    task::spawn(stdin().forward(upload));
                    task::block_on(hashes.forward(virt::stdout()));
                })?;
            }
            SinkCmd::Launch {
                remote,
                suri,
                robot,
            } => {
                let (submit, hashes) = virt::launch(remote, suri, robot)?;
                task::spawn(stdin().map(|m| m.map(|s| s == "ON")).forward(submit));
                let hex_encoded = hashes.map(|r| r.map(|h| hex::encode(h)));
                task::block_on(hex_encoded.forward(virt::stdout()))?;
            }
            #[cfg(feature = "ros")]
            SinkCmd::Ros {
                topic_name,
                queue_size,
            } => {
                let topic = virt::ros(topic_name.as_str(), queue_size)?;
                task::block_on(stdin().forward(topic))?;
            }
        }
        Ok(())
    }
}
