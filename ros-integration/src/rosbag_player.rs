use rosbag::{RosBag, Record};
use rosrust::api::raii::Publisher;
use std::{thread, time};
use std::collections::HashMap;
use std::string::String;
use std::sync::Arc;
use super::ros;

mod msg {
    rosmsg_include!(std_msgs / UInt64, std_msgs / String);
}

#[derive(Debug, PartialEq, Clone)]
pub enum WorkerMsg {
    Stop,
}

struct PublisherDesc{
    publisher: Publisher<msg::std_msgs::String>,
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
                    let topic_publisher = ros::publish(conn.topic).unwrap();
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

                            let dcdc: msg::std_msgs::String = rosrust::RosMsg::decode(iterated_msg.data).unwrap();
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
