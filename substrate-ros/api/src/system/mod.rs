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

use std::sync::Arc;
use sp_runtime::traits;
use sc_rpc::system::helpers::{Health, Properties};
use sc_network::{
    specialization::NetworkSpecialization,
    NetworkService, ExHashT,
};

pub use sc_rpc::system::helpers::SystemInfo;

mod ros_api;

#[derive(Clone)]
pub struct System<B: traits::Block, S: NetworkSpecialization<B>, H: ExHashT + Clone + Sync> {
    info: SystemInfo,
    network: Arc<NetworkService<B, S, H>>,
}

impl<B: traits::Block, S, H> System<B, S, H> where
    S: NetworkSpecialization<B>,
    H: ExHashT + Clone + Sync
{
    pub fn new(
        info: SystemInfo,
        network: Arc<NetworkService<B, S, H>>,
    ) -> Self {
        System { info, network }
    }
}

impl<B: traits::Block, S, H> ros_api::SystemApi for System<B, S, H> where
    S: NetworkSpecialization<B>,
    H: ExHashT + Clone + Sync
{
    fn system_name(&self) -> String {
        self.info.impl_name.clone()
    }

    fn system_version(&self) -> String {
        self.info.impl_version.clone()
    }

    fn system_chain(&self) -> String {
        self.info.chain_name.clone()
    }

    fn system_properties(&self) -> Properties {
        self.info.properties.clone()
    }

    fn system_health(&self) -> Health {
        Health {
            peers: self.network.num_connected(),
            is_syncing: self.network.is_major_syncing(),
            should_have_peers: true, // in practice configurations without peers useless
        }
    }
}
