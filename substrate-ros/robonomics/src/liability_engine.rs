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

use std::{
    fs::File,
    sync::{Arc,
           Mutex},
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
                         StartLiabilityPlayer, StartLiabilityPlayerRes,
                         AddLiability, AddLiabilityRes},
    std_msgs
};
use rosrust::api::error::Error;
use log::error;
use futures::executor::block_on;

use crate::rosbag_player::RosbagPlayer;
const LIABILITY_ADD_SRV_NAME: &str = "/liability/add";
const LIABILITY_START_SRV_NAME: &str = "/liability/start_player";

async fn add_liability(
    liability: Liability,
    known_liabilities: &Arc<Mutex<HashMap<u64, RosbagPlayer>>>
) {
    let ipfs = IpfsClient::default();
    let bag_hash = liability.order.objective;
    let liability_id = liability.id;
    let mut liabilities = known_liabilities.lock().unwrap();

    let (response, _) = ipfs.cat(bag_hash.as_str()).compat().into_future().await;
    if let Some(Ok(content)) = response {
        let bag_file = File::create(bag_hash.as_str()).expect("could not create file");
        let mut buffer = AllowStdIo::new(bag_file);
        buffer.write_all(&content).await;
        buffer.close().await;

        let mut player = RosbagPlayer::new(bag_hash);
        liabilities.insert(liability_id, player);
    } else {
        error!("Unable to get IPFS content of {}", bag_hash);
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
    -> Result<Vec<rosrust::Service>, Error> {
    let mut services = vec![];

    let liability_players = Arc::new(Mutex::new(HashMap::new()));
    let players01 = liability_players.clone();

    services.push(rosrust::service::<StartLiabilityPlayer, _>(LIABILITY_START_SRV_NAME, move |req| {
        let mut res = StartLiabilityPlayerRes::default();
        block_on(launch_liability_player(req.id, &players01));
        res.success = true;
        Ok(res)
    })?);

    let players02 = liability_players.clone();
    services.push(rosrust::service::<AddLiability, _>(LIABILITY_ADD_SRV_NAME, move |req| {
        let mut res = AddLiabilityRes::default();
        block_on(add_liability(req.liability, &players02));
        res.success = true;
        Ok(res)
    })?);

    Ok(services)
}
