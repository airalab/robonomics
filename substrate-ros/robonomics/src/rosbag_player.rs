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
use rosbag::{RosBag, Record, RecordsIterator};
use futures_timer::Delay;
use std::time;
use log::debug;
use log::info;

use msgs::std_msgs;

use futures::{prelude::*, io::AllowStdIo, compat::Stream01CompatExt, Poll};
use rosbag::record_types::{MessageData, Connection};
use rosrust::api::Ros;

#[derive(Debug, PartialEq, Clone)]
pub enum WorkerMsg {
    Stop,
}

pub struct RosbagPlayer<T> where
    T: rosrust::Message
{
    bag: RosBag,
    publisher: Publisher<T>,
    topic_conn_ids: Vec<u32>,
    start_msg_timestamp: u64,
    //messages: Vec<PlayerMessageData<'a>>,
    //records: &'a RecordsIterator<'a>
}

type PlayerConnections<'a> = HashMap<u32, Connection<'a>>;
type PlayerMessages<'a> = HashMap<u32, Vec<PlayerMessageData<'a>>>;
type StorageTopics = HashMap<(String, String), Vec<u32>>;

use std::sync::Arc;
use futures::task::Context;
use futures::io::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub struct PlayerMessageData<'a> {
    /// ID for connection on which message arrived
    pub conn_id: u32,
    /// Time at which the message was received in nanoseconds of UNIX epoch
    pub time: u64,
    /// Serialized message data in the ROS serialization format
    pub data: &'a [u8],
}

const MSG_STRING_TYPE: &str = "std_msgs/String";

pub fn construct_player(path: String) -> Result<impl Future<Output=()>, Error> where
{
    let bag = RosBag::new(path.as_str()).unwrap();
    let records = bag.records();

    let mut player_connections = PlayerConnections::new();
    let mut player_messages = PlayerMessages::new();
    //let mut player_messages = HashMap::new();
    let mut storage_topics = StorageTopics::new();
    //let mut storage_topics = HashMap::new();

    let mut start_msg_timestamp: u64 = 0;

    records.for_each(|record|
        match record {
            Ok(Record::Connection(conn)) => {
                let conn01 = conn.clone();
                player_connections.insert(conn01.id.clone(), conn01.clone());

                match storage_topics.get_mut(&(conn01.storage_topic.to_string(), conn01.tp.to_string())) {
                    Some(ids) => ids.push(conn01.id.clone()),
                    None => ({
                        let ids= vec![conn01.id.clone()];
                        storage_topics.insert((conn01.storage_topic.to_string(), conn01.tp.to_string()), ids);
                    }),
                }
            },

            Ok(Record::MessageData(msg_data)) => {
                if start_msg_timestamp == 0 || msg_data.time < start_msg_timestamp {
                    start_msg_timestamp = msg_data.time;
                }
            }

            _ => ()
        }
    );

    let mut players = vec![];
    for ((storage_topic_name, topic_type), topics_ids) in storage_topics.iter() {
        match topic_type.as_str() {
            MSG_STRING_TYPE => players.push(RosbagPlayer::<std_msgs::String>::new(storage_topic_name, topics_ids.clone(), path.clone(), start_msg_timestamp.clone()).play()),
            _ => println!("Unsupported topic type")
        }
    }
    return Ok(future::join_all(players).map(|_| ()))
}

impl<T> RosbagPlayer<T> where
    T: rosrust::rosmsg::RosMsg + rosrust::Message
{

    pub fn new(topic: &str, topic_conn_ids: Vec<u32>, path: String, start_msg_timestamp: u64) -> Self {
        println!("Construct Player with {:?} {:?}", topic, topic_conn_ids);
        let publisher = rosrust::publish(topic, 32).unwrap();
        //let messages : Vec<MessageData<>> = vec![];
        let bag = RosBag::new(path.as_str()).unwrap();
        //let player = Self{ bag: bag, publisher: publisher, messages: messages};
        let player = Self{ bag: bag, publisher: publisher, topic_conn_ids: topic_conn_ids, start_msg_timestamp: start_msg_timestamp};
        player
    }

    pub async fn play(mut self) {
        // create low-level iterator over rosbag records
        let mut prev_msg_timestamp: u64 = self.start_msg_timestamp;
        for record in self.bag.records() {
            match record {
                Ok(Record::Chunk(chunk)) => {
                    //let mut prev_msg_timestamp: u64 = 0;
                    for inner in chunk.iter_msgs() {
                        if let Ok(msg) = inner {
                            if self.topic_conn_ids.contains(&msg.conn_id) {
                                let dcdc: T = rosrust::RosMsg::decode(msg.data).unwrap();
                                //let publisher_description = self.map.get_mut(&msg.conn_id).unwrap();
                                //debug!("rosbag_player {}: publish msg {:?} with decoded data {:?} of type {} into topic {}",
                                //    self.path, msg, dcdc.data, publisher_description.msgtype, publisher_description.rostopic);
                                let sleep_time_duration = {
                                    // sleep 5 seconds before 1st message (giving chance to register publishers on master)
                                    // sleep (current msg timestamp - previous msg timestamp) nanoseconds for all following messages

                                    if prev_msg_timestamp != self.start_msg_timestamp {
                                        time::Duration::from_nanos(msg.time - prev_msg_timestamp)

                                    } else {
                                        time::Duration::from_secs(5)
                                    }
                                };
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
