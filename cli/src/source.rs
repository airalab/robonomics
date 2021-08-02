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
//! Robonomics data source interface.

#![deny(missing_docs)]

use crate::error::Result;
use futures::prelude::*;
use robonomics_io::sink::virt::stdout;
use robonomics_io::source::{serial, virt};
use robonomics_protocol::pubsub::Multiaddr;
use sp_core::crypto::Ss58AddressFormat;
use std::{convert::TryFrom, time::Duration};
use structopt::clap::arg_enum;
use async_std::task;

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
    /// Reading datalog.
    Datalog {
        /// Robonomics node API endpoint.
        #[structopt(long, value_name = "REMOTE_URI", default_value = "ws://127.0.0.1:9944")]
        remote: String,
        /// Reader account seed URI.
        #[structopt(short, value_name = "ADDRESS")]
        address: String,
        //TODO: follow flag
    },
    /// Download data from IPFS storage.
    Ipfs {
        /// IPFS node endpoint.
        #[structopt(
            long,
            value_name = "REMOTE_URI",
            default_value = "http://127.0.0.1:5001"
        )]
        remote: String,
    },
    /// Robot launch request events.
    Launch {
        /// Robonomics node API endpoint.
        #[structopt(long, default_value = "ws://127.0.0.1:9944")]
        remote: String,
        /// Output address format.
        #[structopt(
            long,
            short = "n",
            possible_values = &Ss58AddressFormat::all_names()[..],
            parse(try_from_str = Ss58AddressFormat::try_from),
            case_insensitive = true,
            default_value = "robonomics",
        )]
        network: Ss58AddressFormat,
    },
    #[cfg(feature = "ros")]
    /// Subscribe for data from ROS topic.
    Ros {
        /// Topic name.
        #[structopt(value_name = "TOPIC_NAME")]
        topic_name: String,
        /// Queue size.
        #[structopt(long, default_value = "10")]
        queue_size: usize,
    },
    /// request-response server
    #[structopt(name = "reqres")]
    ReqRes {
        /// multiaddress of server, i.e. /ip4/192.168.0.102/tcp/61241
        #[structopt(value_name = "MULTIADDR")]
        address: String,

        /// server peer ID, i.e. 12D3KooWHdqJNpszJR4na6pheUwSMNQCuGXU6sFTGDQMyQWEsszS
        #[structopt(value_name = "PEER_ID")]
        peerid: String,

        /// request type: `ping` or `get`
        #[structopt(value_name = "METHOD")]
        method: String,

        /// value: only required when `method` is `get`
        #[structopt(name = "VALUE", required_if("method", "get"))]
        value: Option<String>,
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
                                Encoding::Debug => format!("{}", msg),
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
            SourceCmd::Datalog { remote, address } => {
                let data = virt::datalog(remote, address)?;
                task::block_on(
                    data.map(|msg| {
                        msg.map(|rec| {
                            rec.iter()
                                .map(|item| {
                                    format!(
                                        "{:?}\n",
                                        String::from_utf8(item.1.to_vec())
                                            .unwrap_or("<no string>".to_string())
                                    )
                                })
                                .collect()
                        })
                    })
                    .forward(stdout()),
                )?;
            }
            SourceCmd::Ipfs { remote } => {
                let (download, data) = virt::ipfs(remote.as_str()).expect("ipfs launch");
                task::spawn(virt::stdin().forward(download));
                task::block_on(
                    data.map(|m| {
                        m.map(|msg| String::from_utf8(msg).unwrap_or("<no string>".to_string()))
                    })
                    .forward(stdout()),
                )?;
            }
            SourceCmd::Launch { remote, network } => {
                let stream = task::block_on(virt::launch(remote, network))?;
                task::block_on(
                    stream
                        .map(|(sender, robot, param)| {
                            Ok(format!("{} >> {} : {}", sender, robot, param))
                        })
                        .forward(stdout()),
                )?;
            }
            #[cfg(feature = "ros")]
            SourceCmd::Ros {
                topic_name,
                queue_size,
            } => {
                let (topic, _sub) = virt::ros(topic_name.as_str(), queue_size)?;
                task::block_on(topic.map(|msg| Ok(msg)).forward(stdout()))?;
            }
            SourceCmd::ReqRes {
                address,
                peerid,
                method,
                value,
            } => {
                let (_err, res) = virt::reqres(address, peerid, method, value)?;
                task::block_on(res.forward(stdout()))?;
            }
        }
        Ok(())
    }
}
