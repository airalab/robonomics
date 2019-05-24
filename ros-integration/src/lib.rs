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

use log::debug;
use std::sync::Arc;
use futures::{Future, Stream, sync::mpsc};
use network::SyncProvider;
use keystore::Store as Keystore;
use client::{
    Client, CallExecutor, BlockchainEvents,
    blockchain::HeaderBackend,
    backend::Backend
};
use runtime_primitives::{
    codec::{Decode, Encode, Compact},
    generic::{BlockId, Era},
    traits::{As, Block, Header, BlockNumberToHash}
};
use primitives::{
    Blake2Hasher, H256, twox_128,
    sr25519, crypto::Pair, crypto::Ss58Codec, 
    storage::{StorageKey, StorageData, StorageChangeSet}
};
use transaction_pool::txpool::{ChainApi, Pool, ExtrinsicFor};
use robonomics_runtime::{
    AccountId, Call, UncheckedExtrinsic, EventRecord, Event,
    robonomics::*, RobonomicsCall, Nonce, Runtime
};

mod msg;
mod ipfs;
mod rosbag_player;

use msg::{std_msgs, robonomics_msgs};
use rosbag_player::RosbagPlayer;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

/// Simple liability engine.
fn liability_stream(
    stream: mpsc::UnboundedReceiver<robonomics_msgs::Liability>,
    ros_account: String,
) -> impl Future<Item=(),Error=()> {
    stream
        .filter(move |liability| liability.promisor == ros_account)
        .for_each(|liability| {
            ipfs::read_file(liability.order.objective.as_str()).then(move |_| {
                let player = RosbagPlayer::new(liability.order.objective.as_str());
                player.play_rosbag().map_err(|_| ())
            })
        })
}

/// Robonomics extrinsic sender.
fn extrinsic_stream<B, E, P, RA>(
    client: Arc<Client<B, E, P::Block, RA>>,
    key: sr25519::Pair,
    pool: Arc<Pool<P>>,
    stream: mpsc::UnboundedReceiver<RobonomicsCall<Runtime>>
) -> impl Future<Item=(),Error=()> where
    B: Backend<P::Block, Blake2Hasher>,
    E: CallExecutor<P::Block, Blake2Hasher> + Send + Sync,
    P: ChainApi,
    RA: Send + Sync,
    P::Block: Block<Hash=H256>,
{
    // Get account address from keypair
    let local_id: AccountId = key.public();
    let mut nonce_key = b"System AccountNonce".to_vec();
    nonce_key.extend_from_slice(&local_id.0[..]);
    let storage_key = StorageKey(twox_128(&nonce_key[..]).to_vec());

    stream.for_each(move |call| {
        let block_id = BlockId::hash(client.backend().blockchain().info().unwrap().best_hash);
        let nonce = if let Some(storage_data) = client.storage(&block_id, &storage_key).unwrap() {
            let nonce: Nonce = Decode::decode(&mut &storage_data.0[..]).unwrap();
            Compact::<Nonce>::from(nonce)
        } else {
            Compact::<Nonce>::from(0)
        };
        let payload = (
            nonce,
            Call::Robonomics(call),
            Era::immortal(),
            client.genesis_hash(),
        );
        let signature = key.sign(&payload.encode());
        let extrinsic = UncheckedExtrinsic::new_signed(
            payload.0.into(),
            payload.1,
            local_id.clone().into(),
            signature.into(),
            payload.2,
        );
        let xt: ExtrinsicFor<P> = Decode::decode(&mut &extrinsic.encode()[..]).unwrap();
        let res = pool.submit_one(&block_id, xt);
        debug!("submission result: {:?}", res); 
        Ok(())
    })
}

/// Storage event listener.
fn event_stream<B, C>(
    client: Arc<C>,
    liability_tx: mpsc::UnboundedSender<robonomics_msgs::Liability>,
) -> impl Future<Item=(),Error=()> where
    C: BlockchainEvents<B>,
    B: Block,
{
    let demand_pub = rosrust::publish("liability/demand/incoming", QUEUE_SIZE).unwrap();
    let offer_pub = rosrust::publish("liability/offer/incoming", QUEUE_SIZE).unwrap();
    let liability_pub = rosrust::publish("liability/incoming", QUEUE_SIZE).unwrap();

    let events_key = StorageKey(twox_128(b"System Events").to_vec());
    client.storage_changes_notification_stream(Some(&[events_key])).unwrap()
        .map(|(block, changes)| StorageChangeSet { block, changes: changes.iter().cloned().collect()})
        .for_each(move |change_set| {
            // Decode events from change set
            let records: Vec<Vec<EventRecord<Event>>> = change_set.changes.iter()
                .filter_map(|(_, mbdata)| if let Some(StorageData(data)) = mbdata {
                    Decode::decode(&mut &data[..])
                } else { None })
                .collect();
            let events: Vec<Event> = records.concat().iter().cloned().map(|r| r.event).collect();

            // Iterate and dispatch events
            events.iter().for_each(|event| {
                if let Event::robonomics(e) = event { match e {
                    RawEvent::NewDemand(hash, demand) => {
                        debug!("NewDemand: {:?} {:?}", hash, demand);
                        let mut msg = robonomics_msgs::Demand::default();
                        let model = bs58::encode(&demand.order.model);
                        let objective = bs58::encode(&demand.order.objective);

                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = demand.order.cost.to_string();
                        msg.sender          = demand.sender.to_ss58check();

                        demand_pub.send(msg).unwrap();
                    },

                    RawEvent::NewOffer(hash, offer) => {
                        debug!("NewOffer: {:?} {:?}", hash, offer);
                        let mut msg = robonomics_msgs::Offer::default();
                        let model = bs58::encode(&offer.order.model);
                        let objective = bs58::encode(&offer.order.objective);

                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = offer.order.cost.to_string();
                        msg.sender          = offer.sender.to_ss58check();

                        offer_pub.send(msg).unwrap();
                    },

                    RawEvent::NewLiability(id, liability) => {
                        debug!("NewLiability: {:?} {:?}", id, liability);
                        let mut msg = robonomics_msgs::Liability::default();
                        let model = bs58::encode(&liability.order.model);
                        let objective = bs58::encode(&liability.order.objective);

                        msg.id              = *id;
                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = liability.order.cost.to_string();
                        msg.promisee        = liability.promisee.to_ss58check();
                        msg.promisor        = liability.promisor.to_ss58check();

                        liability_pub.send(msg.clone()).unwrap();
                        
                        // Send new liability to engine
                        liability_tx.unbounded_send(msg).unwrap();
                    },

                    _ => ()
                }
            }
        });
        Ok(())
    })
}

/// Robonomics node status.
fn status_stream<B, C, N>(
    client: Arc<C>,
    network: Arc<N>,
) -> impl Future<Item=(),Error=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    N: SyncProvider<B>,
    B: Block,
{
    let hash_pub = rosrust::publish("blockchain/best_hash", QUEUE_SIZE).unwrap();
    let number_pub = rosrust::publish("blockchain/best_number", QUEUE_SIZE).unwrap();
    let peers_pub = rosrust::publish("network/peers", QUEUE_SIZE).unwrap();

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

/// ROS API main routine.
pub fn start_ros_api<N, B, E, P, RA>(
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
    rosrust::try_init_with_options("robonomics", false);
    ipfs::init();

    let keystore_default_public = &keystore.contents().unwrap()[0];
    let keystore_default_key = keystore.load(&keystore_default_public, &String::new()).unwrap();
    let key = sr25519::Pair::from_seed(*keystore_default_key.seed());
    let ros_account = key.public().to_ss58check();
    println!("ROS account: {:?}", ros_account);

    // Create extrinsics channel
    let (demand_tx, extrinsic_rx) = mpsc::unbounded();
    let offer_tx = demand_tx.clone();
    let finalize_tx = demand_tx.clone();

    // Subscribe for sending demand extrinsics
    let demand = rosrust::subscribe("liability/demand/send", QUEUE_SIZE, move |v: robonomics_msgs::Order| {
        let model = bs58::decode(v.model).into_vec().unwrap();
        let objective = bs58::decode(v.objective).into_vec().unwrap();
        let cost = v.cost.parse().unwrap();
        demand_tx.unbounded_send(RobonomicsCall::demand(model, objective, cost)).unwrap();
    }).expect("failed to create demand subscriber");

    // Subscribe for sending offer extrinsics
    let offer = rosrust::subscribe("liability/offer/send", QUEUE_SIZE, move |v: robonomics_msgs::Order| {
        let model = bs58::decode(v.model).into_vec().unwrap();
        let objective = bs58::decode(v.objective).into_vec().unwrap();
        let cost = v.cost.parse().unwrap();
        offer_tx.unbounded_send(RobonomicsCall::offer(model, objective, cost)).unwrap();
    }).expect("failed to create demand subscriber");

    // Finalize liability
    let finalize = rosrust::subscribe("liability/finalize", QUEUE_SIZE, move |v: robonomics_msgs::Finalize| {
        let result = bs58::decode(v.result).into_vec().unwrap();
        finalize_tx.unbounded_send(RobonomicsCall::finalize(v.id, result)).unwrap();
    }).expect("failed to create liability subscriber");

    // Create liability channel
    let (liability_tx, liability_rx) = mpsc::unbounded();

    // Store subscribers in vector
    let subs = vec![demand, offer, finalize];

    extrinsic_stream(client.clone(), key, pool, extrinsic_rx)
        .join(liability_stream(liability_rx, ros_account))
        .join(event_stream(client.clone(), liability_tx))
        .join(status_stream(client, network))
        .map(|_| ())
        .select(on_exit)
        .then(move |_| {
            // move subscribers to this closure
            // subscribers must not be removed until exit
            subs;
            Ok(())
        })
}
