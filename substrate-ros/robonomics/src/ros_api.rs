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
use futures::{
    prelude::*,
    channel::mpsc,
    StreamExt as _,
};
use client::{
    Client, CallExecutor, BlockchainEvents,
    backend::Backend,
};
use runtime_primitives::{
    codec::{Decode, Encode, Compact},
    generic::{BlockId, Era},
    traits::{Block, BlockNumberToHash},
};
use primitives::{
    Blake2Hasher, H256, twox_128,
    sr25519, crypto::Pair, crypto::Ss58Codec,
    storage::{StorageKey, StorageData},
};
use transaction_pool::txpool::{ChainApi, Pool, ExtrinsicFor};
use robonomics_runtime::{
    AccountId, Call, UncheckedExtrinsic, EventRecord, Event, Hash,
    robonomics::*, RobonomicsCall, Nonce, Runtime
};

use msgs::substrate_ros_msgs;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

/// Robonomics extrinsic sender.
fn extrinsic_stream<B, E, P, RA>(
    client: Arc<Client<B, E, P::Block, RA>>,
    key: sr25519::Pair,
    pool: Arc<Pool<P>>,
    stream: mpsc::UnboundedReceiver<RobonomicsCall<Runtime>>
) -> impl Future<Output=()> where
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
        let block_id = BlockId::hash(client.info().chain.best_hash);
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
        debug!("txpool submit result: {:?}", res); 
        future::ready(())
    })
}

/// Storage event listener.
fn event_stream<B, C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<B>,
    B: Block,
{
    let demand_pub = rosrust::publish("liability/demand/incoming", QUEUE_SIZE).unwrap();
    let offer_pub = rosrust::publish("liability/offer/incoming", QUEUE_SIZE).unwrap();
    let liability_pub = rosrust::publish("liability/incoming", QUEUE_SIZE).unwrap();

    let events_key = StorageKey(twox_128(b"System Events").to_vec());
    client.storage_changes_notification_stream(Some(&[events_key]), None).unwrap()
        .for_each(move |(_, changes)| {
            // Decode events from change set
            let records: Vec<Vec<EventRecord<Event, Hash>>> = changes.iter()
                .filter_map(|(_, _, mbdata)| if let Some(StorageData(data)) = mbdata {
                    Decode::decode(&mut &data[..])
                } else { None })
                .collect();
            let events: Vec<Event> = records.concat().iter().cloned().map(|r| r.event).collect();

            // Iterate and dispatch events
            for event in events {
                if let Event::robonomics(e) = event { match e {
                    RawEvent::NewDemand(hash, demand) => {
                        debug!("NewDemand: {:?} {:?}", hash, demand);
                        let mut msg = substrate_ros_msgs::Demand::default();
                        let model = bs58::encode(&demand.order.model);
                        let objective = bs58::encode(&demand.order.objective);

                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = demand.order.cost.to_string();
                        msg.sender          = demand.sender.to_ss58check();

                        demand_pub.send(msg).expect("Unable to send NewDemand event message");
                    },

                    RawEvent::NewOffer(hash, offer) => {
                        debug!("NewOffer: {:?} {:?}", hash, offer);
                        let mut msg = substrate_ros_msgs::Offer::default();
                        let model = bs58::encode(&offer.order.model);
                        let objective = bs58::encode(&offer.order.objective);

                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = offer.order.cost.to_string();
                        msg.sender          = offer.sender.to_ss58check();

                        offer_pub.send(msg).expect("Unable to send NewOffer event message");
                    },

                    RawEvent::NewLiability(id, liability) => {
                        debug!("NewLiability: {:?} {:?}", id, liability);
                        let mut msg = substrate_ros_msgs::Liability::default();
                        let model = bs58::encode(&liability.order.model);
                        let objective = bs58::encode(&liability.order.objective);

                        msg.id              = id;
                        msg.order.model     = model.into_string();
                        msg.order.objective = objective.into_string();
                        msg.order.cost      = liability.order.cost.to_string();
                        msg.promisee        = liability.promisee.to_ss58check();
                        msg.promisor        = liability.promisor.to_ss58check();

                        liability_pub.send(msg.clone()).expect("Unable to send NewLiability event message");
                    },

                    _ => ()
                }}
            }
            future::ready(())
        })
}

/// ROS API main routine.
pub fn start_api<B, E, P, RA>(
    client: Arc<Client<B, E, P::Block, RA>>,
    pool: Arc<Pool<P>>,
    ros_key: sr25519::Pair,
) -> (impl Future<Output=()> + 'static, Vec<rosrust::Subscriber>) where
    B: Backend<P::Block, Blake2Hasher> + 'static,
    E: CallExecutor<P::Block, Blake2Hasher> + Send + Sync + 'static,
    P: ChainApi + 'static,
    RA: Send + Sync + 'static,
    P::Block: Block<Hash=H256>,
{
    rosrust::try_init_with_options("robonomics", false);

    let ros_account = ros_key.public().to_ss58check();
    println!("ROS account: {:?}", ros_account);

    // Create extrinsics channel
    let (demand_tx, extrinsic_rx) = mpsc::unbounded();
    let offer_tx = demand_tx.clone();
    let finalize_tx = demand_tx.clone();

    // Subscribe for sending demand extrinsics
    let demand = rosrust::subscribe("liability/demand/send", QUEUE_SIZE, move |v: substrate_ros_msgs::Order| {
        let model = bs58::decode(v.model).into_vec().unwrap();
        let objective = bs58::decode(v.objective).into_vec().unwrap();
        let cost = v.cost.parse().unwrap();
        demand_tx.unbounded_send(RobonomicsCall::demand(model, objective, cost, None)).unwrap();
    }).expect("failed to create demand subscriber");

    // Subscribe for sending offer extrinsics
    let offer = rosrust::subscribe("liability/offer/send", QUEUE_SIZE, move |v: substrate_ros_msgs::Order| {
        let model = bs58::decode(v.model).into_vec().unwrap();
        let objective = bs58::decode(v.objective).into_vec().unwrap();
        let cost = v.cost.parse().unwrap();
        offer_tx.unbounded_send(RobonomicsCall::offer(model, objective, cost, None)).unwrap();
    }).expect("failed to create demand subscriber");

    // Finalize liability
    let finalize = rosrust::subscribe("liability/finalize", QUEUE_SIZE, move |v: substrate_ros_msgs::Finalize| {
        let result = bs58::decode(v.result).into_vec().unwrap();
        finalize_tx.unbounded_send(RobonomicsCall::finalize(v.id, result)).unwrap();
    }).expect("failed to create liability subscriber");

    // Store subscribers in vector
    let subscriptions = vec![demand, offer, finalize];
    let extrinsics = extrinsic_stream(client.clone(), ros_key, pool, extrinsic_rx);
    let events = event_stream(client);

    (future::join(extrinsics, events).map(|_| ()), subscriptions)
}
