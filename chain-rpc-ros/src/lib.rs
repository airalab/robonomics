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

use log::debug;
use std::sync::Arc;
use futures::{Future, Stream};
use network::SyncProvider;
use keystore::Store as Keystore;
use client::{
    Client, CallExecutor, BlockchainEvents,
    blockchain::HeaderBackend,
    backend::Backend
};
use runtime_primitives::{
    traits::{As, Block, Header}
};
use primitives::{
    Blake2Hasher, H256, twox_128
};
use transaction_pool::txpool::{ChainApi, Pool};
use robonomics_runtime::{
};
use std::thread;
use futures::future::IntoFuture;
use log::info;

mod msg;

use msg::{std_msgs, robonomics_msgs};

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;
const BEST_HASH_ROS_NAME: &str = "blockchain/best_hash";
const BEST_NUMBER_ROS_NAME: &str = "blockchain/best_number";
const FINALIZED_HASH_ROS_NAME: &str = "blockchain/finalized_hash";
const FINALIZED_NUMBER_ROS_NAME: &str = "blockchain/finalized_number";
const NETWORK_PEERS_ROS_NAME: &str = "network/peers";

/// Robonomics node status.
fn status_stream<B, C, N>(
    client: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    N: SyncProvider<B>,
    B: Block,
{
    let hash_pub = rosrust::publish(BEST_HASH_ROS_NAME, QUEUE_SIZE).unwrap();
    let number_pub = rosrust::publish(BEST_NUMBER_ROS_NAME, QUEUE_SIZE).unwrap();
    let peers_pub = rosrust::publish(NETWORK_PEERS_ROS_NAME, QUEUE_SIZE).unwrap();

    client.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut hash_msg = std_msgs::String::default(); 
            hash_msg.data = block.header.hash().to_string();
            hash_pub.send(hash_msg).unwrap();

            let mut peers_msg = std_msgs::UInt64::default();
            peers_msg.data = network.peers().len() as u64;
            peers_pub.send(peers_msg).unwrap();

            let mut number_msg = std_msgs::UInt64::default();
		    number_msg.data = block.header.number().as_();
            number_pub.send(number_msg).unwrap();
        }
        Ok(())
    })
}

/// Robonomics node status.
fn finality_stream<B, C, N>(
    client: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    N: SyncProvider<B>,
    B: Block,
{
    let finalized_number_pub = rosrust::publish(FINALIZED_NUMBER_ROS_NAME, QUEUE_SIZE).unwrap();
    let finalized_hash_pub = rosrust::publish(FINALIZED_HASH_ROS_NAME, QUEUE_SIZE).unwrap();

    client.finality_notification_stream().for_each(move |block| {
        let mut finalized_number_msg = std_msgs::UInt64::default();
        finalized_number_msg.data = block.header.number().as_();
        finalized_number_pub.send(finalized_number_msg).unwrap();

        let mut finalized_hash_msg = std_msgs::String::default();
        finalized_hash_msg.data = block.header.hash().to_string();
        finalized_hash_pub.send(finalized_hash_msg).unwrap();

        Ok(())
    })
}

fn rpc_api_stream<B, C, N>(
    client_original: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> + 'static where
    C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
    N: SyncProvider<B>,
    B: Block,
{
    thread::spawn(move || {
        info!("Start rosrust service");

        let client = client_original.clone();
        let _best_hash_service =
            rosrust::service::<msg::robonomics_msgs::BlockHash, _>(BEST_HASH_ROS_NAME, move |_req| {
                // Callback for handling requests
                let mut res = msg::robonomics_msgs::BlockHashRes::default();
                res.hash = client.info().unwrap().best_hash.to_string();
                debug!("rosservice get best hash res {}", res.hash);
                Ok(res)
            });

        let client = client_original.clone();
        let _best_number_service =
            rosrust::service::<msg::robonomics_msgs::BlockNumber, _>(BEST_NUMBER_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockNumberRes::default();
                res.number = client.info().unwrap().best_number.as_();
                debug!("rosservice get best number res {}", res.number);
                Ok(res)
            });

        let client = client_original.clone();
        let _finalized_hash_service =
            rosrust::service::<msg::robonomics_msgs::BlockHash, _>(FINALIZED_HASH_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockHashRes::default();
                res.hash = client.info().unwrap().finalized_hash.to_string();
                debug!("rosservice get finalized hash res {}", res.hash);
                Ok(res)
            });

        let client = client_original.clone();
        let _finalized_number_service =
            rosrust::service::<msg::robonomics_msgs::BlockNumber, _>(FINALIZED_NUMBER_ROS_NAME, move |_req| {
                let mut res = msg::robonomics_msgs::BlockNumberRes::default();
                res.number = client.info().unwrap().finalized_number.as_();
                debug!("rosservice get finalized number res {}", res.number);
                Ok(res)
            });
        // Block the thread until a shutdown signal is received
        rosrust::spin();
    });
    Ok(()).into_future()
}

/// ROS API status routine
pub fn start_status_api<N, B, E, P, RA>(
    network: Arc<N>,
    client: Arc<Client<B, E, P::Block, RA>>,
    pool: Arc<Pool<P>>,
    keystore: &Keystore,
    on_exit: impl Future<Item=(),Error=()> + 'static,
) -> impl Future<Item=(),Error=()> + 'static where
    N: SyncProvider<P::Block> + 'static,
    B: Backend<P::Block, Blake2Hasher> + 'static,
    E: CallExecutor<P::Block, Blake2Hasher> + Send + Sync + 'static,
    P: ChainApi + 'static,
    RA: Send + Sync + 'static,
    P::Block: Block<Hash=H256>,
{
    status_stream(client.clone(), network.clone())
        .join(finality_stream(client.clone(), network.clone()))
        .join(rpc_api_stream(client, network))
        .map(|_| ())
        .select(on_exit)
        .then(move |_| {
            Ok(())
        })
}
