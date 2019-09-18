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
                         StartLiabilityPlayer, StartLiabilityPlayerRes},
};
use rosrust::api::error::Error;
use log::error;
use futures::executor::block_on;

use crate::rosbag_player::RosbagPlayer;
use crate::rosbag_player::construct_player;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

const LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME: &str = "/liability/prepare";
const LIABILITY_READY_TOPIC_NAME: &str = "liability/ready";
const LIABILITY_START_SRV_NAME: &str = "/liability/start";

use msgs::std_msgs;
use futures::channel::mpsc;
use std::sync::RwLock;

use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;

fn add_liability_stream(
    stream: mpsc::UnboundedReceiver<(Liability, Receiver<()>)>,
) -> impl Future<Output=()> + 'static {

    let ipfs = IpfsClient::default();
    let liability_ready_pub = rosrust::publish(LIABILITY_READY_TOPIC_NAME, QUEUE_SIZE).unwrap();

    stream.for_each_concurrent( 10, move |(liability, l_lock)| {
        let l = liability.clone();
        let bag_hash = liability.order.objective;
        let liability_id = liability.id;

        println!("Received liability {:?}", l);
        let rbplayer = construct_player("/tmp/".to_owned() + &bag_hash).unwrap();
        println!("construct with {:?}", bag_hash);
        liability_ready_pub.send(l.clone()).unwrap();
        l_lock.then(|_| rbplayer)
    })
}

pub fn start_liability_engine()
    -> Result<(impl Future<Output=()> + 'static, Vec<rosrust::Service>, Vec<rosrust::Subscriber>), Error> {
    let mut services = vec![];
    let mut subscribers = vec![];

    let (liability_tx, liability_rx) = mpsc::unbounded::<(Liability, Receiver<()>)>();
    let add_liabilities_stream = add_liability_stream(liability_rx);

    let locks_hash_map00 = Arc::new(Mutex::new(HashMap::new()));
    let locks01 = Arc::clone(&locks_hash_map00);
    let locks02 = Arc::clone(&locks_hash_map00);

    subscribers.push(
        rosrust::subscribe(LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME, QUEUE_SIZE, move |l: Liability| {
            let (locks_tx, locks_rx) = oneshot::channel();
            let mut lhm = locks01.lock().unwrap();
            if ! lhm.contains_key(&l.id) {
                liability_tx.unbounded_send((l.clone(), locks_rx)).unwrap();
                lhm.insert(l.id, locks_tx);
            }
        }).expect("failed to create incoming liability subscriber")
    );

    services.push(rosrust::service::<StartLiabilityPlayer, _>(LIABILITY_START_SRV_NAME, move |req| {
        let mut res = StartLiabilityPlayerRes::default();
        let mut lhm = locks02.lock().unwrap();
        let lock_sender = lhm.remove(&req.id).unwrap();
        lock_sender.send(());
        res.success = true;
        Ok(res)
    })?);

    Ok((add_liabilities_stream.map(|_| ()), services, subscribers))
}
