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

use rosrust::api::raii::Publisher;
use std::collections::HashMap;
use rosbag::{RosBag, Record};
use std::{thread, time};
use std::string::String;
use std::sync::Arc;
use log::debug;

use crate::msg::std_msgs;

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
    bag: Arc<RosBag>,
    map: Publishers,
}


impl RosbagPlayer{
    pub fn new(path: &str) -> Self {
        let t_path = path;
        let tbag = RosBag::new(t_path);
        let bag = tbag.unwrap();

        let mut player = RosbagPlayer {
            path: path.to_string(),
            map: HashMap::new(),
            bag: Arc::new(bag),
        };
        player.register_topics();
        player
    }


    fn register_topics(&mut self) {
        let bag = &self.bag;
        let mut records = bag.records();

        let header = match records.next() {
            Some(Ok(Record::BagHeader(bh))) => bh,
            _ => panic!("Failed to acquire bag header record"),
        };

        records.seek(header.index_pos).unwrap();
        for record in records {
            let iterated = record.unwrap();

            match iterated {
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
    }

    pub fn play_rosbag(mut self) -> Result<(), rosbag::Error> {
        // create low-level iterator over rosbag records
        let bag = self.bag.clone();
        thread::spawn(move || -> Result<(), rosbag::Error> {
            let mut records = bag.records();
            // get first `Chunk` record and iterate over `Message` records in it
            for record in &mut records {
                let iterated = record?;
                match iterated {
                    Record::Chunk(chunk) => {
                        let mut prev_msg_timestamp: u64 = 0;
                        for msg in chunk.iter_msgs() {
                            let iterated_msg = msg?;

                            let dcdc: std_msgs::String = rosrust::RosMsg::decode(iterated_msg.data).unwrap();
                            let publisher_description = self.map.get_mut(&iterated_msg.conn_id).unwrap();
                            debug!("rosbag_player {}: publish msg {:?} with decoded data {:?} of type {} into topic {}",
                                   self.path, iterated_msg, dcdc.data, publisher_description.msgtype, publisher_description.rostopic);
                            let sleep_time_duration;
                            {
                                // sleep 5 seconds before 1st message (giving chance to register publishers on master)
                                // sleep (current msg timestamp - previous msg timestamp) nanoseconds for all following messages
                                if prev_msg_timestamp != 0 {
                                    sleep_time_duration = time::Duration::from_nanos(iterated_msg.time - prev_msg_timestamp)
                                } else {
                                    prev_msg_timestamp = iterated_msg.time;
                                    sleep_time_duration = time::Duration::from_secs(5);
                                }
                            }
                            thread::sleep(sleep_time_duration);
                            publisher_description.publisher.send(dcdc).unwrap();
                        }
                        break;
                    }
                    _ => (),
                }
            }
            Ok(())
        });
        Ok(())
    }
}
