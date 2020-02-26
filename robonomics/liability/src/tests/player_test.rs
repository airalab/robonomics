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

use std::sync::Arc;
use std::path::{PathBuf};
use futures::executor::ThreadPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time};
use std::marker::PhantomData;

use msgs::std_msgs;
use substrate_ros_robonomics::rosbag_player::build_players;

struct PlayerTestHelper<T> where
    T: rosrust::Message
{
    topic: String,
    bag_path: PathBuf,
    message_catches: Arc<AtomicBool>,
    message_type: PhantomData<T>
}

impl<T> PlayerTestHelper<T> where
    T: rosrust::rosmsg::RosMsg + rosrust::Message
{
    pub fn new(topic: &str, rosbag_name: &str) -> PlayerTestHelper<T> {
        let message_catches = Arc::new(AtomicBool::new(false));

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/");
        path.push(rosbag_name);

        Self {
            topic: topic.into(),
            bag_path: path,
            message_catches: message_catches,
            message_type: PhantomData
        }
    }

    pub fn perform_test(self) -> bool {
        let mic = self.message_catches.clone();
        let topic = self.topic.clone();

        thread::spawn(move || {
            let subscriber = rosrust::subscribe(topic.as_str(), 10, move |_data: T| {
                mic.store(true, Ordering::Relaxed);
            });
            rosrust::spin();
        });

        let ones = time::Duration::from_secs(1);
        thread::sleep(ones);

        let bag_path = self.bag_path.as_path().to_str().unwrap();
        let rbplayer = build_players(bag_path);

        match rbplayer {
            Ok(player) => {
                log::info!("Construct player for {:?}", bag_path);
                ThreadPool::new().expect("Failed to create threadpool").run(player);
                self.message_catches.load(Ordering::Relaxed)
            },
            Err(e) => {
                log::error!("Failed to construct player with error {}", e);
                false
            }
        }
    }
}

#[test]
fn test_player_for_types() {
    rosrust::try_init_with_options("robonomics_player_test", false);
    assert!(rosrust::is_initialized());

    let t1 = PlayerTestHelper::<std_msgs::Bool>::new("/testtopics/bool", "rosbag-std_msgs-Bool.bag");
    assert!(t1.perform_test());

    let t2 = PlayerTestHelper::<std_msgs::String>::new("/testtopics/string", "rosbag-std_msgs-String.bag");
    assert!(t2.perform_test());

    let t3 = PlayerTestHelper::<std_msgs::UInt64>::new("/testtopics/uint64", "rosbag-std_msgs-UInt64.bag");
    assert!(t3.perform_test());

    let t4 = PlayerTestHelper::<std_msgs::Time>::new("/testtopics/time", "rosbag-std_msgs-Time.bag");
    assert!(t4.perform_test());

}
