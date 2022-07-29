///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2022 Robonomics Network <research@robonomics.network>
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
//! Robonomics Publisher/Subscriber API.
//!
//! A basic pubsub client demonstrating libp2p and the gossipsub protocol.
//!
//! Using two terminal windows, start two instances.
//! ```sh
//! target/debug/robonomics --dev --tmp -l rpc=trace
//! target/debug/robonomics --dev --tmp --ws-port 9991 -l rpc=trace
//! ```
//!
//! Then using two terminal windows, start two clients.
//! One of them will send messages, the other one will catch them and print.
//! You can of course open more terminal windows and add more participants.
//!
//! Pubsub subscribe:
//!
//! ```{python}
//! import time
//! from robonomicsinterface import PubSub, Account
//!
//! def subscription_handler(obj, update_nr, subscription_id):
//!     print(obj['params']['result'])
//!     if update_nr >= 2:
//!         return 0
//!
//! account = Account(remote_ws="ws://127.0.0.1:9944")
//! pubsub = PubSub(account)
//!
//! print(pubsub.listen("/ip4/127.0.0.1/tcp/44440"))
//! time.sleep(2)
//! print(pubsub.connect("/ip4/127.0.0.1/tcp/44441"))
//! time.sleep(2)
//! print(pubsub.subscribe("topic_name", result_handler=subscription_handler))
//! ```
//!
//! Pubsub publish:
//!
//! ```{python}
//! import time
//! from robonomicsinterface import PubSub
//!
//! account = Account(remote_ws="ws://127.0.0.1:9991")
//! pubsub = PubSub(account)
//!
//! print(pubsub.listen("/ip4/127.0.0.1/tcp/44441"))
//! time.sleep(2)
//! print(pubsub.connect("/ip4/127.0.0.1/tcp/44440"))
//! time.sleep(2)
//!
//! for i in range(10):
//!     time.sleep(2)
//!     print("publish:", pubsub.publish("topic_name", "message_" + str(time.time())))
//! ```

use robonomics_protocol::pubsub::PubSub;

use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    PendingSubscription,
};
use libp2p::Multiaddr;

pub struct PubSubRpc<T: PubSub>(T);

#[rpc(server)]
pub trait PubSubRpc {
    /// Returns local peer ID.
    #[method(name = "pubsub_peer")]
    fn peer_id(&self) -> RpcResult<String>;

    /// Listen address for incoming connections.
    #[method(name = "pubsub_listen")]
    async fn listen(&self, address: Multiaddr) -> RpcResult<bool>;

    /// Returns a list of node addresses.
    #[method(name = "pubsub_listeners")]
    async fn listeners(&self) -> RpcResult<Vec<Multiaddr>>;

    /// Connect to peer and add it into swarm.
    #[method(name = "pubsub_connect")]
    async fn connect(&self, address: Multiaddr) -> RpcResult<bool>;

    /// Subscribe for a topic with given name.
    #[subscription(name = "pubsub_subscribe", unsubscribe = "pubsub_unsubscribe", item = crate::pubsub::Message)]
    fn subscribe(&self, topic_name: String);

    /// Publish message into the topic by name.
    #[method(name = "pubsub_publish")]
    async fn publish(&self, topic_name: String, message: String) -> RpcResult<bool>;
}

#[async_trait]
impl<T: PubSub> PubSubRpcServer for PubSubRpc<T> {
    fn peer_id(&self) -> RpcResult<String> {
        Ok(self.0.peer_id().to_string())
    }

    async fn listen(&self, address: Multiaddr) -> RpcResult<bool> {
        self.0.listen(address).await.or(Ok(false))
    }

    async fn listeners(&self) -> RpcResult<Vec<Multiaddr>> {
        self.0.listeners().await.or(Ok(vec![]))
    }

    async fn connect(&self, address: Multiaddr) -> RpcResult<bool> {
        self.0.connect(address).await.or(Ok(false))
    }

    fn subscribe(&self, pending: PendingSubscription, topic_name: String) {
        let mut sink = pending.accept().unwrap();
        let inbox = self.0.subscribe(&topic_name.clone());
        tokio::spawn(async move {
            sink.pipe_from_stream(inbox).await;
        });
    }

    async fn publish(&self, topic_name: String, message: String) -> RpcResult<bool> {
        self.0
            .publish(&topic_name, message.as_bytes().to_vec())
            .await
            .or(Ok(false))
    }
}
