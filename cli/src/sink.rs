///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
        /// RWS subscription address.
        #[structopt(long, value_name = "RWS_ADDRESS")]
        rws: Option<String>,
    },
    /// Upload data into IPFS storage.
    Ipfs {
        /// IPFS node endpoint.
        #[structopt(
            long,
            value_name = "REMOTE_URI",
            default_value = "http://127.0.0.1:5001"
        )]
        remote: String,
    },
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
        /// RWS subscription address.
        #[structopt(long, value_name = "RWS_ADDRESS")]
        rws: Option<String>,
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
    /// request-response client
    #[structopt(name = "reqres")]
    ReqRes {
        /// multiaddress of server, i.e. /ip4/192.168.0.102/tcp/61241
        #[structopt(value_name = "MULTIADDR", default_value = "/ip4/0.0.0.0/tcp/0")]
        address: String,
    },
}

impl SinkCmd {
    /// Write data into sink device.
    pub fn run(&self) -> Result<()> {
        match self.clone() {
            SinkCmd::PubSub {
                topic_name,
                listen,
                hearbeat_secs,
            } => {
                let hearbeat = Duration::from_secs(hearbeat_secs);
                let pubsub = virt::pubsub(listen, topic_name, hearbeat)?;
                task::block_on(stdin().forward(pubsub))?;
            }
            SinkCmd::Datalog { remote, suri, rws } => {
                let (submit, hashes) = virt::datalog(remote, suri, rws)?;
                task::spawn(stdin().forward(submit));
                let hex_encoded = hashes.map(|r| r.map(|h| hex::encode(h)));
                task::block_on(hex_encoded.forward(virt::stdout()))?;
            }
            SinkCmd::Ipfs { remote } => {
                let (upload, hashes) = virt::ipfs(remote.as_str()).expect("ipfs launch");
                task::spawn(stdin().forward(upload));
                task::block_on(hashes.forward(virt::stdout()))?;
            }
            SinkCmd::Launch {
                remote,
                suri,
                robot,
                rws,
            } => {
                let (submit, hashes) = virt::launch(remote, suri, robot, rws)?;
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
            SinkCmd::ReqRes { address } => {
                let val = virt::reqres(address.as_str().to_string())?;
                task::block_on(val.map(|msg| Ok(msg)).forward(virt::stdout()))?;
            }
        }
        Ok(())
    }
}
