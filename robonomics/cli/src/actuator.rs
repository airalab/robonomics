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
///! Robonomics I/O CLI interface.

use crate::error::Result;
use robonomics_protocol::pubsub::Multiaddr;
use robonomics_io::sensor::virt::Stdin;
use robonomics_io::actuator::virt;
use robonomics_io::Consumer;
use async_std::task;

#[derive(structopt::StructOpt, Clone, Debug)]
pub enum ActuatorCmd {
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
    }
}

impl ActuatorCmd {
    pub fn run(&self) -> Result<()> {
        let stdin = Stdin::new();
        match self.clone() {
            ActuatorCmd::PubSub { topic_name, listen, bootnodes } => {
                let device = virt::PubSub::new(listen, bootnodes, topic_name).unwrap();
                Ok(task::block_on(device.consume(Box::new(stdin))))
            }
        }
    }
}
