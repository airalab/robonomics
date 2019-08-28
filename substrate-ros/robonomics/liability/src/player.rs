///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
///! Generic Rust implementation of ROSBAG player.
/// http://wiki.ros.org/rosbag/Commandline#rosbag_play

use rosrust::api::raii::Publisher;
use std::collections::HashMap;
use rosbag::{RosBag, Record};
use futures_timer::Delay;
use std::time;
use log::debug;

use msgs::std_msgs;

#[derive(Debug, PartialEq, Clone)]
pub enum WorkerMsg {
    Stop,
}

struct PublisherDesc{
    publisher: Publisher<std_msgs::String>,
    rostopic: String,
    msgtype: String,
}

type Publishers = HashMap<u32, PublisherDesc>;

pub struct RosbagPlayer {
    path: String,
    bag: RosBag,
    map: Publishers,
}

impl RosbagPlayer{
    pub fn new(path: String) -> Self {
        let bag = RosBag::new(path.as_str()).unwrap();
        let mut player = RosbagPlayer { path, map: HashMap::new(), bag };
        player.register_topics();
        player
    }


    fn register_topics(&mut self) -> Result<(), rosbag::Error> {
        let mut records = self.bag.records();

        let header = match records.next() {
            Some(Ok(Record::BagHeader(bh))) => bh,
            _ => panic!("Failed to acquire bag header record"),
        };

        records.seek(header.index_pos)?;
        for record in records {
            match record? {
                Record::Connection(conn) => {
                    let topic_publisher = rosrust::publish(conn.topic, 32).unwrap();
                    let p_desc = PublisherDesc {
                        publisher: topic_publisher,
                        rostopic: conn.topic.to_string(),
                        msgtype: conn.tp.to_string()
                    };
                    self.map.insert(conn.id, p_desc);
                    debug!("rosbag_player {}: id {} with topic {} of type {} inserted to publishers map",
                          self.path, conn.id, conn.topic, conn.tp);
                }
                _ => ()
            }
        };
        Ok(())
    }

    pub async fn play(&mut self) {
        // create low-level iterator over rosbag records
        for record in self.bag.records() {
            match record {
                Ok(Record::Chunk(chunk)) => {
                    let mut prev_msg_timestamp: u64 = 0;
                    for inner in chunk.iter_msgs() {
                        if let Ok(msg) = inner {
                            let dcdc: std_msgs::String = rosrust::RosMsg::decode(msg.data).unwrap();
                            let publisher_description = self.map.get_mut(&msg.conn_id).unwrap();
                            debug!("rosbag_player {}: publish msg {:?} with decoded data {:?} of type {} into topic {}",
                                self.path, msg, dcdc.data, publisher_description.msgtype, publisher_description.rostopic);
                            let sleep_time_duration = {
                                // sleep 5 seconds before 1st message (giving chance to register publishers on master)
                                // sleep (current msg timestamp - previous msg timestamp) nanoseconds for all following messages
                                if prev_msg_timestamp != 0 {
                                    time::Duration::from_nanos(msg.time - prev_msg_timestamp)
                                } else {
                                    prev_msg_timestamp = msg.time;
                                    time::Duration::from_secs(5)
                                }
                            };
                            Delay::new(sleep_time_duration).await;
                            publisher_description.publisher.send(dcdc).unwrap();
                    }}
                },
                _ => ()
            }
        }
    }
}
