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
//! Robonomics network layer.

use futures::Future;
use std::{sync::Arc, time::Duration};

use crate::{error::Result, network::worker::NetworkWorker, pubsub::Pubsub};

pub mod behaviour;
pub mod discovery;
pub mod worker;

pub struct RobonomicsNetwork {
    pub pubsub: Arc<Pubsub>,
}

impl RobonomicsNetwork {
    pub fn new(
        heartbeat_interval: u64,
        bootnodes: Vec<String>,
        disable_mdns: bool,
        disable_kad: bool,
    ) -> Result<(Arc<Self>, impl Future<Output = ()>)> {
        let mut network_worker = NetworkWorker::new(
            Duration::from_millis(heartbeat_interval),
            disable_mdns,
            disable_kad,
        )?;

        // Reach out to another nodes if specified
        discovery::add_explicit_peers(&mut network_worker.swarm, bootnodes, disable_kad);

        Ok((
            Arc::new(Self {
                pubsub: network_worker.service.clone(),
            }),
            network_worker,
        ))
    }
}
