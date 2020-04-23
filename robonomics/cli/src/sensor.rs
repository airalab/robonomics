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
use robonomics_io::{Sensor, Actuator};
use robonomics_io::sensor::{virt, serial};
use robonomics_io::actuator::virt::Stdout;
use async_std::task;
use futures::StreamExt;

#[derive(structopt::StructOpt, Clone, Debug)]
pub enum SensorCmd {
    /// Nova SDS011 particle sensor.
    SDS011 {
        /// Serial port that sensor connected for.
        #[structopt(long, default_value = "/dev/ttyUSB0")]
        port: String,
        /// Request interval in minutes.
        #[structopt(long, default_value = "5")]
        period: u8,
    },
    /// Subscribe for broadcasing data.
    #[structopt(name = "pubsub")]
    PubSub {
        /// Subscribe for given topic name and print received messages.
        topic_name: String,
        /// Listen address for incoming connections. 
        #[structopt(
            long,
            value_name = "TOPIC_NAME",
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

impl SensorCmd {
    pub fn run(&self) -> Result<()> {
        let stdout: Stdout = Actuator::new(()).unwrap();
        match self.clone() {
            SensorCmd::SDS011 { port, period } => {
                let config = serial::SDS011Config { port, period };
                let sensor = serial::SDS011::new(config).unwrap();
                let measure = sensor.read().map(|m| format!("{:?}", m));
                task::block_on(stdout.write(Box::new(measure)))
            }
            SensorCmd::PubSub { topic_name, listen, bootnodes } => {
                let config = virt::PubSubConfig { topic_name, listen, bootnodes };
                let sensor = virt::PubSub::new(config).unwrap();
                let measure = sensor.read().map(|m|
                    format!(
                        "{}: {}",
                        m.from.to_base58(),
                        String::from_utf8_lossy(&m.data[..]),
                    )
                );
                task::block_on(stdout.write(Box::new(measure)))
            }

        }
        Ok(())
    }
}
