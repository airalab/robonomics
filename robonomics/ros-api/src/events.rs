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
//! This module publish Robonomics runtime events into ROS namespace.

use sp_core::{
    twox_128,
    storage::{StorageKey, StorageData},
    crypto::Ss58Codec
};
use rosrust::api::error::Error as RosError;
use sp_runtime::traits::{self, Header};
use futures_util::stream::StreamExt;
use base58::{FromBase58, ToBase58};
use sp_runtime::generic::BlockId;
use sc_client::BlockchainEvents;
use frame_system::EventRecord;
use sp_api::ProvideRuntimeApi;
use futures::future::Future;
use std::sync::Arc;
use codec::Decode;
use log::debug;

use msgs::robonomics_msgs::{Demand, Offer, Liability, Report};
use pallet_robonomics_runtime_api::SystemEventsApi;
use pallet_robonomics_liability as liability;
use pallet_robonomics_provider as provider;
use node_runtime::Event;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

/// Finalized event listener.
pub fn finalized_event_stream<B, C>(
    client: Arc<C>,
) -> Result<impl Future<Output=()>, RosError> where 
    B: traits::Block,
    C: BlockchainEvents<B> + ProvideRuntimeApi<B> + 'static,
    C::Api: SystemEventsApi<B, Event>,
{
    let demand_pub    = rosrust::publish("liability/demand", QUEUE_SIZE)?;
    let offer_pub     = rosrust::publish("liability/offer", QUEUE_SIZE)?;
    let liability_pub = rosrust::publish("liability/new", QUEUE_SIZE)?;
    let report_pub    = rosrust::publish("liability/report", QUEUE_SIZE)?;

    let stream = client.finality_notification_stream()
        .for_each(move |notify| {
            let block_id = BlockId::Hash(notify.hash);
            let events = client
                .runtime_api()
                .events(&block_id)
                .expect("Runtime communication error");
            debug!("Events at {}: {:?}", notify.hash, events);

            // Iterate and dispatch events
            for event in events { match event {
                Event::pallet_robonomics_provider(e) => match e {
                    provider::RawEvent::NewDemand(technics, economics, sender) => {
                        debug!("NewDemand: {:?} {:?} from {:?}", technics, economics, sender);

                        let mut msg = Demand::default();
                        msg.technics  = technics.to_base58();
                        msg.sender    = sender.to_ss58check();
                        demand_pub
                            .send(msg)
                            .expect("unable to publish ROS message");
                    },

                    provider::RawEvent::NewOffer(technics, economics, sender) => {
                        debug!("NewOffer: {:?} {:?} from {:?}", technics, economics, sender);

                        let mut msg = Offer::default();
                        msg.technics  = technics.to_base58();
                        msg.sender    = sender.to_ss58check();
                        offer_pub
                            .send(msg)
                            .expect("unable to publish ROS message");
                    },
                },

                Event::pallet_robonomics_liability(e) => match e {
                    liability::RawEvent::NewLiability(index, technics, economics, promisee, promisor) => {
                        debug!("NewLiability: {:?} {:?} {:?} {:?} {:?}",
                               index, technics, economics, promisee, promisor);

                        let mut msg = Liability::default();
                        msg.index     = index;
                        msg.technics  = technics.to_base58().to_string();
                        msg.promisee  = promisee.to_ss58check();
                        msg.promisor  = promisor.to_ss58check();
                        liability_pub
                            .send(msg)
                            .expect("unable to publish ROS message");
                    },

                    liability::RawEvent::NewReport(index, report) => {
                        debug!("NewReport: {:?} {:?}", index, report);

                        let mut msg = Report::default();
                        msg.index  = index;
                        msg.report = report.to_base58().to_string();
                        report_pub
                            .send(msg)
                            .expect("unable to publish ROS message");
                    },
                },

                _ => ()
            }}

            futures::future::ready(())
        });
    Ok(stream)
}
