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
///! Robonomics liability engine.
///
/// This module helps robot to get liability metadata and content from different networks.
/// And handle it correctly according to specified liability lifecycle, including persistent
/// liability tracking, replaying and supervising.

use std::{
    fs::File,
    sync::{Arc, Mutex},
    collections::HashMap,
};
use ipfs_api::IpfsClient;
use futures::{
    prelude::*,
    io::AllowStdIo,
    compat::Stream01CompatExt,
};
use msgs::{
    substrate_ros_msgs::{Liability,
                         StartLiabilityPlayer, StartLiabilityPlayerRes},
};
use rosrust::api::error::Error;
use log::error;
use futures::executor::block_on;

use crate::rosbag_player::RosbagPlayer;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

const LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME: &str = "/liability/prepare";
const LIABILITY_READY_TOPIC_NAME: &str = "liability/ready";
const LIABILITY_START_SRV_NAME: &str = "/liability/start";

async fn add_liability(
    liability: Liability,
    known_liabilities: &Arc<Mutex<HashMap<u64, RosbagPlayer>>>,
    publisher: &Arc<rosrust::Publisher<Liability>>
) {
    let ipfs = IpfsClient::default();
    let l = liability.clone();
    let bag_hash = liability.order.objective;
    let liability_id = liability.id;
    let mut liabilities = known_liabilities.lock().unwrap();

    if ! (liabilities.contains_key(&liability_id)) {
        let (response, _) = ipfs.cat(bag_hash.as_str()).compat().into_future().await;
        if let Some(Ok(content)) = response {
            let bag_file = File::create(bag_hash.as_str()).expect("could not create file");
            let mut buffer = AllowStdIo::new(bag_file);
            buffer.write_all(&content).await;
            buffer.close().await;

            let player = RosbagPlayer::new(bag_hash);
            liabilities.insert(liability_id, player);
            publisher.send(l).unwrap();
        } else {
            error!("Unable to get IPFS content of {}", bag_hash);
        }
    }

}

/// Simple liability engine.
async fn launch_liability_player(
    liability_id: u64,
    known_liabilities: &Arc<Mutex<HashMap<u64, RosbagPlayer>>>
) {
    let mut data = known_liabilities.lock().unwrap();
    let mut player = data.remove(&liability_id).unwrap();
    player.play().await;
}

pub fn start_liability_engine()
    -> Result<(Vec<rosrust::Service>, Vec<rosrust::Subscriber>), Error> {
    let mut services = vec![];
    let mut subscribers = vec![];

    let liability_players = Arc::new(Mutex::new(HashMap::new()));
    let liability_ready_pub = Arc::new(rosrust::publish(LIABILITY_READY_TOPIC_NAME, QUEUE_SIZE).unwrap());

    let players01 = liability_players.clone();
    subscribers.push(
        rosrust::subscribe(LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME, QUEUE_SIZE, move |l: Liability| {
            block_on(add_liability(l.clone(), &players01, &liability_ready_pub));
        }).expect("failed to create incoming liability subscriber")
    );

    let players02 = liability_players.clone();
    services.push(rosrust::service::<StartLiabilityPlayer, _>(LIABILITY_START_SRV_NAME, move |req| {
        let mut res = StartLiabilityPlayerRes::default();
        block_on(launch_liability_player(req.id, &players02));
        res.success = true;
        Ok(res)
    })?);

    Ok((services, subscribers))
}
