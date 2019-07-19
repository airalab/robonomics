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
use rosrust::api::error::Error;
use crate::traits::RosRpc;
use runtime_primitives::traits;
use substrate_rpc::system::helpers::{Health, Properties};
use network::{
    specialization::NetworkSpecialization,
    NetworkService, ExHashT,
};
use msgs::{
    std_srvs::{Trigger, TriggerRes},
    substrate_ros_msgs::{SystemHealth, SystemHealthRes}
};

pub use substrate_rpc::system::helpers::SystemInfo;

const SYSTEM_NAME_SRV_NAME: &str = "/system/name";
const SYSTEM_VERSION_SRV_NAME: &str = "/system/version";
const SYSTEM_CHAIN_SRV_NAME: &str = "/system/chain_name";
const SYSTEM_PROPERTIES_SRV_NAME: &str = "/system/properties";
const SYSTEM_HEALTH_SRV_NAME: &str = "/system/health";

pub struct System<B: traits::Block, S: NetworkSpecialization<B>, H: ExHashT> {
    info: SystemInfo,
    network: Arc<NetworkService<B, S, H>>,
}

impl<B: traits::Block, S, H> System<B, S, H> where
    S: NetworkSpecialization<B>,
    H: ExHashT 
{
    pub fn new(
        info: SystemInfo,
        network: Arc<NetworkService<B, S, H>>,
    ) -> Self {
        System { info, network }
    }

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

impl<B: traits::Block, S, H> RosRpc for System<B, S, H> where
    S: NetworkSpecialization<B>,
    H: ExHashT 
{
    fn start(api: Arc<Self>) -> Result<Vec<rosrust::Service>, Error> {
        let mut services = vec![];

        let api1 = api.clone();
        services.push(
            rosrust::service::<Trigger, _>(SYSTEM_NAME_SRV_NAME, move |_| {
                let mut res = TriggerRes::default();
                res.success = true;
                res.message = api1.system_name(); 
                Ok(res)
            })?
        );

        let api2 = api.clone();
        services.push(
            rosrust::service::<Trigger, _>(SYSTEM_VERSION_SRV_NAME, move |_| {
                let mut res = TriggerRes::default();
                res.success = true;
                res.message = api2.system_version(); 
                Ok(res)
            })?
        );

        let api3 = api.clone();
        services.push(
            rosrust::service::<Trigger, _>(SYSTEM_CHAIN_SRV_NAME, move |_| {
                let mut res = TriggerRes::default();
                res.success = true;
                res.message = api3.system_chain();
                Ok(res)
            })?
        );

        let api4 = api.clone();
        services.push(
            rosrust::service::<Trigger, _>(SYSTEM_PROPERTIES_SRV_NAME, move |_| {
                let mut res = TriggerRes::default();
                res.success = true;
                res.message = serde_json::to_string(&api4.system_properties()).unwrap();
                Ok(res)
            })?
        );

        let api5 = api.clone();
        services.push(
            rosrust::service::<SystemHealth, _>(SYSTEM_HEALTH_SRV_NAME, move |_| {
                let mut res = SystemHealthRes::default();
                let health = api5.system_health();
                res.peers = health.peers as u32;
                res.is_syncing = health.is_syncing;
                Ok(res)
            })?
        );

        Ok(services)
    }
}
