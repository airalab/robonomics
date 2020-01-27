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
//! This module publish Robonomics runtime messages into ROS namespace.

use msgs::robonomics_msgs::{Demand, Offer, Liability, Report};
use pallet_robonomics_agent_runtime_api::RobonomicsLiabilityApi;
use pallet_robonomics_liability::traits::Technical;
use pallet_robonomics_liability as liability;
use pallet_robonomics_provider as provider;
use rosrust::api::error::Error as RosError;
use sp_runtime::{traits, generic::BlockId};
use futures_util::stream::StreamExt;
use sc_client::BlockchainEvents;
use sp_core::crypto::Ss58Codec;
use sp_api::ProvideRuntimeApi;
use futures::future::Future;
use base58::ToBase58;
use std::sync::Arc;
use log::debug;

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

#[cfg(not(feature = "liability"))]
pub fn receive_stream<C>(_client: C)
    -> Result<impl Future<Output=()>, String> { Ok(futures::future::ready(())) }

/// Finalized event listener.
#[cfg(feature = "liability")]
pub fn receive_stream<B: traits::Block, T, C>(
    client: Arc<C>,
) -> Result<impl Future<Output=()>, String> where 
    T: liability::Trait,
    <T as frame_system::Trait>::AccountId: Ss58Codec,
    <<T as liability::Trait>::Technics as Technical>::Parameter: AsRef<[u8]>,
    C: BlockchainEvents<B> + ProvideRuntimeApi<B> + 'static,
    C::Api: RobonomicsLiabilityApi<B, T>,
{
    let demand_pub    = rosrust::publish("liability/demand", QUEUE_SIZE)
        .map_err(|e| format!("ROS error: {}", e))?;
    let offer_pub     = rosrust::publish("liability/offer", QUEUE_SIZE)
        .map_err(|e| format!("ROS error: {}", e))?;
    //let liability_pub = rosrust::publish("liability/new", QUEUE_SIZE)?;
    //let report_pub    = rosrust::publish("liability/report", QUEUE_SIZE)?;

    let stream = client.finality_notification_stream()
        .for_each(move |notify| {
            let block_id = BlockId::Hash(notify.hash);
            let messages = client
                .runtime_api()
                .recv(&block_id)
                .expect("Runtime communication error");

            // Iterate and dispatch events
            for msg in messages { match msg {
                provider::RobonomicsMessage::Demand(provider::Order { technics, economics, sender, .. }) => {
                    debug!("New message => Demand {:?} {:?} from {:?}", technics, economics, sender);

                    let mut msg  = Demand::default();
                    msg.technics = technics.as_ref().to_base58();
                    msg.sender   = sender.to_ss58check();
                    demand_pub
                        .send(msg)
                        .expect("unable to publish ROS message");
                },

                provider::RobonomicsMessage::Offer(provider::Order { technics, economics, sender, .. }) => {
                    debug!("New message => Offer {:?} {:?} from {:?}", technics, economics, sender);

                    let mut msg  = Offer::default();
                    msg.technics = technics.as_ref().to_base58();
                    msg.sender   = sender.to_ss58check();
                    offer_pub
                        .send(msg)
                        .expect("unable to publish ROS message");
                },
            }}

            futures::future::ready(())
        });

    Ok(stream)
}
                /*
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
            */
