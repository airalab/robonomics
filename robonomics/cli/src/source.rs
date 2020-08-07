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
//! Robonomics data source interface.

#![deny(missing_docs)]

use crate::error::Result;
use async_std::task;
use futures::prelude::*;
use robonomics_io::sink::virt::stdout;
use robonomics_io::source::{serial, virt};
use robonomics_protocol::pubsub::Multiaddr;
use std::time::Duration;
use structopt::clap::arg_enum;

/// Source device commands.
#[derive(structopt::StructOpt, Clone, Debug)]
pub enum SourceCmd {
    /// Nova SDS011 particle sensor.
    SDS011 {
        /// Serial port that sensor connected for.
        #[structopt(long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Request interval in minutes.
        #[structopt(long, default_value = "5")]
        period: u8,
        /// Source values encoding.
        #[structopt(
            short,
            default_value = "json",
            possible_values = &Encoding::variants(),
            case_insensitive = true,
        )]
        encoding: Encoding,
    },
    /// Subscribe for broadcasing data.
    #[structopt(name = "pubsub")]
    PubSub {
        /// Subscribe for given topic name and print received messages.
        topic_name: String,
        /// Listen address for incoming connections.
        #[structopt(long, value_name = "MULTIADDR", default_value = "/ip4/0.0.0.0/tcp/0")]
        listen: Multiaddr,
        /// Indicates PubSub nodes for first connections.
        #[structopt(long, value_name = "MULTIADDR", use_delimiter = true)]
        bootnodes: Vec<Multiaddr>,
        /// How often node should check another nodes availability, in secs.
        #[structopt(long, value_name = "HEARTBEAT_SECS", default_value = "5")]
        hearbeat: u64,
    },
    /// Download data from IPFS storage.
    Ipfs {
        /// IPFS node API endpoint.
        #[structopt(long, default_value = "http://127.0.0.1:5001")]
        remote: String,
    },
    /// Robot launch request events.
    Launch {
        /// Robonomics node API endpoint.
        #[structopt(long, default_value = "ws://127.0.0.1:9944")]
        remote: String,
    },
}

arg_enum! {
    #[derive(Debug, Clone)]
    pub enum Encoding {
        Csv,
        Hex,
        Json,
        Debug,
    }
}

impl SourceCmd {
    /// Read data from source device.
    pub fn run(&self) -> Result<()> {
        match self.clone() {
            SourceCmd::SDS011 {
                port,
                period,
                encoding,
            } => {
                let sensor = serial::sds011(port, period)?;

                task::block_on(
                    sensor
                        .map(|m| {
                            m.map(|msg| match encoding {
                                Encoding::Csv => {
                                    let mut wtr = csv::WriterBuilder::new()
                                        .has_headers(false)
                                        .from_writer(vec![]);
                                    wtr.serialize(msg).unwrap();
                                    String::from_utf8(wtr.into_inner().unwrap()).unwrap()
                                }
                                Encoding::Hex => hex::encode(bincode::serialize(&msg).unwrap()),
                                Encoding::Json => serde_json::to_string(&msg).unwrap(),
                                Encoding::Debug => format!("{:?}", msg),
                            })
                        })
                        .forward(stdout()),
                )?;
            }
            SourceCmd::PubSub {
                topic_name,
                listen,
                bootnodes,
                hearbeat,
            } => {
                let pubsub =
                    virt::pubsub(listen, bootnodes, topic_name, Duration::from_secs(hearbeat))?;

                task::block_on(
                    pubsub
                        .map(|m| {
                            m.map(|msg| {
                                String::from_utf8(msg.data).unwrap_or("<no string>".to_string())
                            })
                        })
                        .forward(stdout()),
                )?;
            }
            SourceCmd::Ipfs { remote } => {
                let (download, data) = virt::ipfs(remote.as_str())?;
                task::spawn(virt::stdin().forward(download));
                task::block_on(
                    data.map(|m| {
                        m.map(|msg| String::from_utf8(msg).unwrap_or("<no string>".to_string()))
                    })
                    .forward(stdout()),
                )?;
            }
            SourceCmd::Launch { remote } => {
                task::block_on(
                    virt::launch(remote)
                        .map(|(sender, robot, param)| {
                            Ok(format!("{} >> {} : {}", sender, robot, param))
                        })
                        .forward(stdout()),
                )?;
            }
        }
        Ok(())
    }
}
