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

use log::{debug, info};
use std::sync::Arc;
use rosrust::api::error;
use futures::{prelude::*, channel::mpsc};
use client::{blockchain::HeaderBackend, BlockchainEvents};
use sr_primitives::{
    codec::{Decode, Encode},
    generic::{BlockId, Era},
    traits::{Header, ProvideRuntimeApi},
};
use primitives::{
    blake2_256, twox_128, sr25519,
    storage::{StorageKey, StorageData},
    crypto::Pair, crypto::Ss58Codec,
};
use transaction_pool::txpool::{ChainApi, Pool, ExtrinsicFor};
use node_runtime::{
    Call, UncheckedExtrinsic, EventRecord, Event,
    types::{Block, Hash, AccountNonceApi},
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
fn extrinsic_stream<C, P>(
    client: Arc<C>,
    pool: Arc<Pool<P>>,
    stream: mpsc::UnboundedReceiver<RobonomicsCall<Runtime>>,
    key: sr25519::Pair,
) -> impl Future<Output=()> where
    C: ProvideRuntimeApi + HeaderBackend<Block>,
    P: ChainApi<Block=Block>,
    C::Api: AccountNonceApi<Block>,
{
    stream.for_each(move |call| {
        let api = client.runtime_api();
        let block_id = BlockId::hash(client.info().best_hash);
        // TODO: also check transaction pool for pending txs
        let nonce = api.account_nonce(&block_id, key.public()).unwrap(); 
        let check_version = system::CheckVersion::new();
        let check_genesis = system::CheckGenesis::new();
		let check_era = system::CheckEra::from(Era::Immortal);
		let check_nonce = system::CheckNonce::from(nonce);
		let check_weight = system::CheckWeight::new();
		let take_fees = balances::TakeFees::from(0);

		let extra = (check_version, check_genesis, check_era, check_nonce, check_weight, take_fees); 
		let raw_payload = (Call::Robonomics(call), extra.clone(), client.info().genesis_hash);

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
        let xt: ExtrinsicFor<P> = Decode::decode(&mut extrinsic.as_slice()).unwrap();

        let res = pool.submit_one(&block_id, xt);
        debug!("txpool submit result: {:?}", res); 
        future::ready(())
    })
}

/// Storage event listener.
fn event_stream<C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<Block>,
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
                    if let Ok(res) = Decode::decode(&mut &data[..]) {
                        Some(res)
                    } else { None }
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

fn import_notification_stream<C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<Block> + HeaderBackend<Block>,
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

fn finality_notification_stream<C>(
    client: Arc<C>,
) -> impl Future<Output=()> where
    C: BlockchainEvents<Block> + HeaderBackend<Block>,
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
pub fn start_api<C, P>(
    client: Arc<C>,
    pool: Arc<Pool<P>>,
    key: sr25519::Pair,
) -> Result<(impl Future<Output=()>, Vec<rosrust::Subscriber>), error::Error> where
    C: ProvideRuntimeApi + HeaderBackend<Block> + BlockchainEvents<Block>,
    P: ChainApi<Block=Block>,
    C::Api: AccountNonceApi<Block>,
{
    info!("ROS API account is {:?}", key.public().to_ss58check());
    rosrust::try_init_with_options("robonomics", false)?;

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
    })?;

    // Subscribe for sending offer extrinsics
    let offer = rosrust::subscribe("liability/offer/send", QUEUE_SIZE, move |v: substrate_ros_msgs::Order| {
        let model = bs58::decode(v.model).into_vec().unwrap();
        let objective = bs58::decode(v.objective).into_vec().unwrap();
        let cost = v.cost.parse().unwrap();
        offer_tx.unbounded_send(RobonomicsCall::offer(model, objective, cost, None)).unwrap();
    })?;

    // Finalize liability
    let finalize = rosrust::subscribe("liability/finalize", QUEUE_SIZE, move |v: substrate_ros_msgs::Finalize| {
        let result = bs58::decode(v.result).into_vec().unwrap();
        finalize_tx.unbounded_send(RobonomicsCall::finalize(v.id, result)).unwrap();
    })?;

    // Store subscribers in vector
    let extrinsics = extrinsic_stream(client.clone(), pool, extrinsic_rx, key);
    let events     = event_stream(client.clone());
    let status     = import_notification_stream(client.clone());
    let finality   = finality_notification_stream(client);

    let subscriptions = vec![demand, offer, finalize];
    let task = future::join4(extrinsics, events, status, finality).map(|_| ()); 

    Ok((task, subscriptions))
}
