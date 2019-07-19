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
//! This module exports Robonomics API into ROS namespace.

use std::fs::File;
use ipfs_api::IpfsClient;
use futures::{
    prelude::*,
    io::AllowStdIo,
    compat::Stream01CompatExt,
};
use msgs::substrate_ros_msgs;
use log::error;

use crate::rosbag_player::RosbagPlayer;

/// Simple liability engine.
async fn launch_liability(
    liability: substrate_ros_msgs::Liability
) {
    let ipfs = IpfsClient::default(); 
    let bag_hash = liability.order.objective;

    let (response, _) = ipfs.cat(bag_hash.as_str()).compat().into_future().await;
    if let Some(Ok(content)) = response {
        let bag_file = File::create(bag_hash.as_str()).expect("could not create file");
        let mut buffer = AllowStdIo::new(bag_file);
        buffer.write_all(&content).await;
        buffer.close().await;

        let mut player = RosbagPlayer::new(bag_hash);
        player.play().await;
    } else {
        error!("Unable to get IPFS content of {}", bag_hash);
    }
}

/*
fn start_liability_engine() {
}
*/
