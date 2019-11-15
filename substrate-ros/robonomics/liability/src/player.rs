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
use msgs::std_msgs;

use futures::prelude::*;
use rosbag::record_types::Connection;
use futures::io::Error;
use std::sync::Arc;

use player_codegen::players_builder;
players_builder!(
    // standard ros messages
    std_msgs / String,
    std_msgs / UInt64,
    std_msgs / Bool,
    std_msgs / Time,
);

pub fn build_players(path: &str) -> Result<impl Future<Output=()>, Error> where
{
    RosBag::new(path).map(|rosbag| players_builder(Arc::new(rosbag)))
}

struct RosbagPlayer<T> where
    T: rosrust::Message
{
    bag: Arc<RosBag>,
    publisher: Publisher<T>,
    topic_conn_ids: Vec<u32>,
    start_msg_timestamp: u64,
}

impl<T> RosbagPlayer<T> where
    T: rosrust::rosmsg::RosMsg + rosrust::Message
{
    pub fn new(topic: &str, topic_conn_ids: Vec<u32>, bag: Arc<RosBag>, start_msg_timestamp: u64) -> Self {
        log::debug!("Construct Player with {:?} {:?}", topic, topic_conn_ids);
        let publisher = rosrust::publish(topic, 32).unwrap();
        let player = Self{ bag: bag, publisher: publisher, topic_conn_ids: topic_conn_ids, start_msg_timestamp: start_msg_timestamp};
        player
    }

    pub async fn play(mut self) {
        // create low-level iterator over rosbag records
        let mut prev_msg_timestamp: u64 = self.start_msg_timestamp;
        for record in self.bag.records() {
            match record {
                Ok(Record::Chunk(chunk)) => {
                    for inner in chunk.iter_msgs() {
                        if let Ok(msg) = inner {
                            if prev_msg_timestamp == self.start_msg_timestamp {
                                let initial_sleep = time::Duration::from_secs(5);
                                Delay::new(initial_sleep).await;
                                prev_msg_timestamp = msg.time;
                            }

                            if self.topic_conn_ids.contains(&msg.conn_id) {
                                let dcdc: T = rosrust::RosMsg::decode(msg.data).unwrap();

                                let sleep_time_duration = time::Duration::from_nanos(msg.time - prev_msg_timestamp);
                                prev_msg_timestamp = msg.time;
                                Delay::new(sleep_time_duration).await;

                                self.publisher.send(dcdc).unwrap();
                            }
                        }
                    }
                },
                _ => ()
            }
        }
    }
}
