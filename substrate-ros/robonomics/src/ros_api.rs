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

use log::{
    debug, info
};
use std::sync::Arc;
use futures::{prelude::*, channel::mpsc};
use client::{
    Client, CallExecutor, BlockchainEvents,
    blockchain::HeaderBackend,
    backend::Backend,
};
use runtime_primitives::{
    codec::{Decode, Encode},
    generic::{BlockId, Era},
    traits::{Header, Block, BlockNumberToHash},
};
use primitives::{
    Blake2Hasher, H256, blake2_256, sr25519, twox_128,
    storage::{StorageKey, StorageData},
    crypto::Pair, crypto::Ss58Codec,
};
use transaction_pool::txpool::{ChainApi, Pool, ExtrinsicFor};
use robonomics_runtime::{
    Call, UncheckedExtrinsic, EventRecord, Event,
    types::{AccountId, Hash},
    robonomics::*, RobonomicsCall, Runtime
};

use msgs::substrate_ros_msgs;
use msgs::std_msgs;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

const BEST_HASH_ROS_TOPIC_NAME: &str = "/chain/best_hash";
const BEST_NUMBER_ROS_TOPIC_NAME: &str = "/chain/best_number";
const FINALIZED_HASH_ROS_TOPIC_NAME: &str = "/chain/finalized_hash";
const FINALIZED_NUMBER_ROS_TOPIC_NAME: &str = "/chain/finalized_number";

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
    let storage_key = StorageKey(blake2_256(&nonce_key[..]).to_vec());

    stream.for_each(move |call| {
        let block_id = BlockId::hash(client.info().chain.best_hash);
        let nonce = if let Some(storage_data) = client.storage(&block_id, &storage_key).unwrap() {
            Decode::decode(&mut &storage_data.0[..]).unwrap()
        } else {
            0
        };
		let check_era = system::CheckEra::from(Era::Immortal);
		let check_nonce = system::CheckNonce::from(nonce);
		let check_weight = system::CheckWeight::from();
		let take_fees = balances::TakeFees::from(0);
		let extra = (check_era, check_nonce, check_weight, take_fees);

		let raw_payload = (Call::Robonomics(call), extra.clone(), client.genesis_hash());
		let signature = raw_payload.using_encoded(|payload| if payload.len() > 256 {
			key.sign(&blake2_256(payload)[..])
		} else {
			key.sign(payload)
		});
		let extrinsic = UncheckedExtrinsic::new_signed(
			raw_payload.0,
            key.public().into(),
			signature.into(),
			extra,
		).encode();
        let xt: ExtrinsicFor<P> = Decode::decode(&mut &extrinsic.as_slice()[..]).unwrap();
        let res = pool.submit_one(&block_id, xt);
        debug!("txpool submit result: {:?}", res); 
        future::ready(())
    })
}

/// Storage event listener.
fn event_stream<B, C>(
    client: Arc<C>,
    key: sr25519::Pair,
) -> impl Future<Output=()> where
    C: BlockchainEvents<B>,
    B: Block,
{
    let ros_account = key.public().to_ss58check();

    let demand_pub = rosrust::publish("liability/demand/incoming", QUEUE_SIZE).unwrap();
    let offer_pub = rosrust::publish("liability/offer/incoming", QUEUE_SIZE).unwrap();
    let liability_pub = rosrust::publish("liability/incoming", QUEUE_SIZE).unwrap();
    let liability_prepare_for_execution_pub = rosrust::publish("liability/prepare", QUEUE_SIZE).unwrap();

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

                        if ros_account == liability.promisor.to_ss58check() {
                            info!("Send liability {:?} to liability engine for preparing to execution", id);
                            liability_prepare_for_execution_pub.send(msg.clone()).expect("Unable to send NewLiability event message");
                        }
                    },

                    _ => ()
                }}
            }
            future::ready(())
        })
}

fn import_notification_stream<B, C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    B: Block<Hash=H256>,
    <<B as Block>::Header as Header>::Number: Into<u64>
{
    let hash_pub = rosrust::publish(BEST_HASH_ROS_TOPIC_NAME, QUEUE_SIZE).unwrap();
    let number_pub = rosrust::publish(BEST_NUMBER_ROS_TOPIC_NAME, QUEUE_SIZE).unwrap();

    client.import_notification_stream().for_each(move |block| {
        if block.is_new_best {
            let mut hash_msg = substrate_ros_msgs::BlockHash::default();
            hash_msg.data = block.hash.into();
            hash_pub.send(hash_msg).unwrap();

            let mut number_msg = std_msgs::UInt64::default();
            number_msg.data = (*block.header.number()).into();
            number_pub.send(number_msg).unwrap();
        }
        future::ready(())
    })
}

fn finality_notification_stream<B, C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<B> + HeaderBackend<B>,
    B: Block<Hash=H256>,
    <<B as Block>::Header as Header>::Number: Into<u64>,
{
    let finalized_number_pub = rosrust::publish(FINALIZED_NUMBER_ROS_TOPIC_NAME, QUEUE_SIZE).unwrap();
    let finalized_hash_pub = rosrust::publish(FINALIZED_HASH_ROS_TOPIC_NAME, QUEUE_SIZE).unwrap();

    client.finality_notification_stream().for_each(move |block| {
        let mut finalized_number_msg = std_msgs::UInt64::default();
        finalized_number_msg.data = (*block.header.number()).into();
        finalized_number_pub.send(finalized_number_msg).unwrap();

        let mut finalized_hash_msg = substrate_ros_msgs::BlockHash::default();
        finalized_hash_msg.data = block.hash.into();
        finalized_hash_pub.send(finalized_hash_msg).unwrap();

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
    <<P::Block as Block>::Header as Header>::Number: Into<u64>,
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
    }).expect("failed to create offer subscriber");

    // Finalize liability
    let finalize = rosrust::subscribe("liability/finalize", QUEUE_SIZE, move |v: substrate_ros_msgs::Finalize| {
        let result = bs58::decode(v.result).into_vec().unwrap();
        finalize_tx.unbounded_send(RobonomicsCall::finalize(v.id, result)).unwrap();
    }).expect("failed to create liability subscriber");

    // Store subscribers in vector
    let subscriptions = vec![demand, offer, finalize];
    let extrinsics = extrinsic_stream(client.clone(), ros_key.clone(), pool, extrinsic_rx);
    let events = event_stream(client.clone(), ros_key);
    let status = import_notification_stream(client.clone());
    let finality = finality_notification_stream(client);

    (future::join4(extrinsics, events, status, finality).map(|_| ()), subscriptions)
}
