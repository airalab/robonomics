///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2020 Airalab <research@aira.life> 
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
//! Robonomics Network publisher/subscriber module console interface.

use libp2p::gossipsub::GossipsubEvent;
use libp2p::{Swarm, Multiaddr};
use std::task::{Context, Poll};
use futures::prelude::*;
use async_std::task;

use crate::error::Result;

/// The PubSub command for pubsub router mode.
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct PubSubCmd {
    /// Bind PubSub router to given topic name.
    #[structopt(long, value_name = "TOPIC_NAME")]
    pub topic: String,
    /// Listen address for incoming connections.
    #[structopt(long, value_name = "MULTIADDR")]
    pub listen: Multiaddr,
    /// Indicates node for first connections to build the mesh.
    #[structopt(long, value_name = "MULTIADDR", use_delimiter = true)]
    pub bootnodes: Vec<Multiaddr>,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub shared_params: sc_cli::SharedParams,
}

#[cfg(feature = "cli")]
impl PubSubCmd {
    /// Initialize
    pub fn init(&self, version: &sc_cli::VersionInfo) -> sc_cli::Result<()> {
        self.shared_params.init(version)
    }

    /// Runs the command and node as pubsub router.
    pub fn run(&self) -> Result<()> {
        let mut swarm = crate::pubsub::new_pubsub(self.topic.clone())?;

        let listener = Swarm::listen_on(&mut swarm, self.listen.clone())?;
        log::info!(target: "robonomics-pubsub",
                   "listener for address {} created: {:?}", self.listen, listener);

        for addr in self.bootnodes.clone() {
            Swarm::dial_addr(&mut swarm, addr.clone())?;
        }

        task::block_on(future::poll_fn(move |cx: &mut Context| {
            loop {
                match swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(gossip_event)) => match gossip_event {
                        GossipsubEvent::Message(peer_id, id, message) => log::info!(
                            target: "robonomics-pubsub",
                            "got message: {} with id: {} from peer: {:?}",
                            String::from_utf8_lossy(&message.data),
                            id,
                            peer_id
                        ),
                        _ => {}
                    },
                    Poll::Ready(None) | Poll::Pending => break,
                }
            }

            for a in Swarm::external_addresses(&swarm) {
                log::info!(target: "robonomics-pubsub",
                           "external address {}", a);
            }

            for addr in libp2p::Swarm::listeners(&swarm) {
                log::info!(target: "robonomics-pubsub",
                           "listening on {:?}", addr);
            }

            Poll::Pending
        }))
    }
}
