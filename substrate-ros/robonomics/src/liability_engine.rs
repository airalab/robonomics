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
    future::Either,
    prelude::*,
    io::AllowStdIo,
    compat::Future01CompatExt
};
use msgs::{
    substrate_ros_msgs::{Liability,
                         StartLiabilityPlayer, StartLiabilityPlayerRes},
};
use rosrust::api::error::Error;
use crate::rosbag_player::build_players;

use futures01::Stream;

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::channel::oneshot::Receiver;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

const LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME: &str = "/liability/prepare";
const LIABILITY_READY_TOPIC_NAME: &str = "liability/ready";
const LIABILITY_START_SRV_NAME: &str = "/liability/start";

// we must download liability objective file from IPFS
// and after downloading we can build rosbag player for liability objective
fn create_liability_player_stream(
    stream: mpsc::UnboundedReceiver<(Liability,  Receiver<()>)>,
    ipfs_client: Arc<IpfsClient>
) -> impl Future<Output=()> {

    // rostopic for publishing info about ready-to-start liabilities
    let mut liability_ready_pub = Arc::new(rosrust::publish(LIABILITY_READY_TOPIC_NAME, QUEUE_SIZE).unwrap());

    stream.for_each_concurrent( 10, move |(liability, l_lock)| {
        let l = liability.clone();
        let bag_hash = liability.order.objective.clone();
        let mut status_sender = liability_ready_pub.clone();

        log::debug!("Received liability {:?}", l);
        download_liability_objective(ipfs_client.clone(), l.clone())
            .then(move |_| {
                let rbplayer = build_players(bag_hash.clone().as_str());
                match rbplayer {
                    Ok(player) => {
                        log::info!("Construct player for {:?}", bag_hash);
                        status_sender.send(l.clone());
                        Either::Left(l_lock.then(|_| player))
                    },
                    Err(e) => {
                        log::error!("Failed to construct player for liability {} with error {}", l.id, e);
                        Either::Right(future::ready(()))
                    }
                }
            })
    })
}

// download liability objective Future
// `liability` message and oneshot start receiver will passed for preparing rosbag player future after objective downloading
// otherwise (failed downloading) rosbag player will not be build
async fn download_liability_objective(
    ipfs_client: Arc<IpfsClient>,
    liability: Liability,
) {
    let bag_hash = liability.order.objective.clone();
    let response = ipfs_client.cat(bag_hash.as_str()).concat2().compat().into_future().await;
    match response {
        Ok(bytes) => {
            let bag_file = File::create(bag_hash.as_str()).expect("could not create file");
            let mut buffer = AllowStdIo::new(bag_file);
            buffer.write_all(&bytes).await;
            buffer.close().await;
        },
        Err(e) => {
            log::error!("IPFS: Failed to download file {:?}", e);
        }
    }
}

pub fn start_liability_engine(
    ipfs_client: Arc<IpfsClient>,
) -> Result<(impl Future<Output=()>, Vec<rosrust::Service>, Vec<rosrust::Subscriber>), Error> {
    let mut services = vec![];
    let mut subscribers = vec![];

    let (construct_player_tx, construct_player_rx) = mpsc::unbounded::<(Liability, Receiver<()>)>();
    let liability_player_stream = create_liability_player_stream( construct_player_rx, ipfs_client);

    // hashmap for store liability id -> oneshot start sender
    // rosbag player must be ready for start, but not play messages until /liability/start service will be called
    let locks_map00 = Arc::new(Mutex::new(HashMap::new()));
    let locks_map01 = Arc::clone(&locks_map00);

    // listen rostopic and initiate rosbag player preparation
    subscribers.push(
        rosrust::subscribe(LIABILITY_PREPARE_FOR_EXECUTION_TOPIC_NAME, QUEUE_SIZE, move |l: Liability| {
            let (locks_tx, locks_rx) = oneshot::channel();
            let mut lhm = locks_map00.lock().unwrap();
            if ! lhm.contains_key(&l.id) {
                construct_player_tx.unbounded_send((l.clone(), locks_rx)).unwrap();
                lhm.insert(l.id, locks_tx);
            }
        }).expect("failed to create incoming liability subscriber")
    );

    services.push(rosrust::service::<StartLiabilityPlayer, _>(LIABILITY_START_SRV_NAME, move |req| {
        let mut res = StartLiabilityPlayerRes::default();
        let mut lhm = locks_map01.lock().unwrap();
        // get start sender and send start signal
        match lhm.remove(&req.id) {
            Some(sender) => {
                sender.send(());
                res.success = true;
                res.msg = "Start signal sent successfully".to_string();
            }
            None => {
                res.success = false;
                res.msg = "Unable to find ready to run liability player".to_string();
            }
        }
        Ok(res)
    })?);

    Ok((liability_player_stream.map(|_| ()), services, subscribers))
}
