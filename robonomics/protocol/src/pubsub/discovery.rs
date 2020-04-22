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
///! Robonomics Publisher/Subscriber protocol node discovery extension.

use futures::{Future, FutureExt, StreamExt, future};
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use libp2p::Multiaddr;
use std::sync::Arc;
use super::PubSub;

/// Peer information service message.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DiscoveryMessage {
    peer_id: String,
    timestamp: u64,
    listeners: Vec<Multiaddr>,
}

/// Peer discovery topic name.
pub const DISCOVERY_TOPIC_NAME: &str = "_robonomics_pubsub_peer_discovery";

/// Simple node discovery algorithm.
///
/// 1. All nodes subscribed for DISCOVERY_TOPIC_NAME.
/// 2. Each node periodically send listened addresses into DISCOVERY_TOPIC_NAME.
/// 3. If node received discovery message then try to connect remove node.
pub fn start<T: PubSub>(pubsub: Arc<T>) -> impl Future<Output = ()> {
    future::join(
        // Message broadcasting task 
        discovery(pubsub.clone()),
        // Subscribe for discovery topic and read messages
        pubsub.clone()
              .subscribe(&DISCOVERY_TOPIC_NAME)
              .for_each(move |msg| connect(pubsub.clone(), msg)),
    ).map(|_| ())
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|t| t.as_secs())
        .unwrap_or(0)
}

async fn discovery<T: PubSub>(pubsub: Arc<T>) {
    let minute = Duration::from_secs(60);

    loop {
        if let Ok(listeners) = pubsub.listeners().await {
            if listeners.len() > 0 {
                let message = DiscoveryMessage {
                    peer_id: pubsub.peer_id().to_base58(),
                    timestamp: timestamp(), 
                    listeners,
                };

                pubsub.publish(&DISCOVERY_TOPIC_NAME, bincode::serialize(&message).unwrap());
            }
        }

        // Sleep for 60 seconds
        futures_timer::Delay::new(minute).await;
    }
}

async fn connect<T: PubSub>(pubsub: Arc<T>, msg: super::Message) {
    // Handle only external messages
    if msg.from == pubsub.peer_id() {
        return;
    }

    let decoded: bincode::Result<DiscoveryMessage> = bincode::deserialize(&msg.data[..]);
    match decoded {
        Ok(message) => {
            for addr in message.listeners {
                let _ = pubsub.connect(addr.clone()).await;
            }
        }
        Err(e) => {
            log::error!(
                target: "robonomics-pubsub",
                "Unable to decode discovery message from {}: {}",
                msg.from.to_base58(), e
            );
        }
    }
}
