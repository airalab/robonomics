use crate::pubsub::Inbox;
use crate::pubsub::{Gossipsub, PubSub};
use jsonrpc_derive::rpc;
use std::sync::Arc;

type RpcResult<T> = Result<T, jsonrpc_core::error::Error>;

#[derive(Clone)]
pub struct PubSubApi {
    pub pubsub: Arc<Gossipsub>,
}

impl PubSubApi {
    pub fn new(pubsub: Arc<Gossipsub>) -> Self {
        PubSubApi { pubsub }
    }
}

#[rpc]
pub trait PubSubT {
    /// Subscribe for a topic with given name.
    ///
    /// Returns ???.
    #[rpc(name = "pubsub_subscribe")]
    fn subscribe(&self, topic_name: String) -> RpcResult<Inbox>;

    /// Unsubscribe for incoming messages from topic.
    ///
    /// Returns true when success.???
    #[rpc(name = "pubsub_unsubscribe")]
    fn unsubscribe(&self, topic_name: String) -> RpcResult<bool>;

    /// Publish message into the topic by name.
    #[rpc(name = "pubsub_publish")]
    fn publish(&self, topic_name: String, message: String) -> RpcResult<()>;
}

impl PubSubT for PubSubApi {
    fn subscribe(&self, topic_name: String) -> RpcResult<Inbox> {
        Ok(self.pubsub.subscribe(&topic_name))
    }

    fn unsubscribe(&self, topic_name: String) -> RpcResult<bool> {
        self.pubsub.unsubscribe(&topic_name);
        Ok(true)
    }

    fn publish(&self, topic_name: String, message: String) -> RpcResult<()> {
        self.pubsub
            .publish(&topic_name, message.as_bytes().to_vec());
        Ok(())
    }
}
