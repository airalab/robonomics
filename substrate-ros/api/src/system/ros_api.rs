///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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

use futures::prelude::*;
use futures_timer::Delay;
use msgs::{
    std_msgs,
    std_srvs::{Trigger, TriggerRes},
    substrate_ros_msgs::{SystemHealth, SystemHealthInfo, SystemHealthRes},
};
use rosrust::api::error::Error;
use sc_rpc::system::helpers::Health;
use sp_chain_spec::Properties;
use std::time::Duration;

const SYSTEM_NAME_SRV_NAME: &str = "/system/name";
const SYSTEM_VERSION_SRV_NAME: &str = "/system/version";
const SYSTEM_CHAIN_SRV_NAME: &str = "/system/chain_name";
const SYSTEM_PROPERTIES_SRV_NAME: &str = "/system/properties";
const SYSTEM_HEALTH_SRV_NAME: &str = "/system/health";

const PUBLISH_DELAY: Duration = Duration::from_secs(1);

const SEND_MESSAGE_ERROR: &str = "Unable send message to ROS topic";

/// ROS Pub/Sub queue size.
/// http://wiki.ros.org/roscpp/Overview/Publishers%20and%20Subscribers#Queueing_and_Lazy_Deserialization
const QUEUE_SIZE: usize = 10;

pub trait SystemApi {
    /// Get the node's implementation name. Plain old string.
    fn system_name(&self) -> String;

    /// Get the node implementation's version. Should be a semver string.
    fn system_version(&self) -> String;

    /// Get the chain's type. Given as a string identifier.
    fn system_chain(&self) -> String;

    /// Get a custom set of properties as a JSON object, defined in the chain spec.
    fn system_properties(&self) -> Properties;

    /// Return health status of the node.
    ///
    /// Node is considered healthy if it is:
    /// - connected to some peers (unless running in dev mode)
    /// - not performing a major sync
    fn system_health(&self) -> Health;
}

fn publish_system_name<T>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: SystemApi,
{
    let publisher = rosrust::publish(SYSTEM_NAME_SRV_NAME, QUEUE_SIZE)?;
    let task = async move {
        loop {
            let mut msg = std_msgs::String::default();
            msg.data = api.system_name();
            publisher.send(msg).expect(SEND_MESSAGE_ERROR);
            Delay::new(PUBLISH_DELAY).await;
        }
    };
    Ok(task)
}

fn publish_system_version<T>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: SystemApi,
{
    let publisher = rosrust::publish(SYSTEM_VERSION_SRV_NAME, QUEUE_SIZE)?;
    let task = async move {
        loop {
            let mut msg = std_msgs::String::default();
            msg.data = api.system_version();
            publisher.send(msg).expect(SEND_MESSAGE_ERROR);
            Delay::new(PUBLISH_DELAY).await;
        }
    };
    Ok(task)
}

fn publish_system_chain<T>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: SystemApi,
{
    let publisher = rosrust::publish(SYSTEM_CHAIN_SRV_NAME, QUEUE_SIZE)?;
    let task = async move {
        loop {
            let mut msg = std_msgs::String::default();
            msg.data = api.system_chain();
            publisher.send(msg).expect(SEND_MESSAGE_ERROR);
            Delay::new(PUBLISH_DELAY).await;
        }
    };
    Ok(task)
}

fn publish_system_health<T>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: SystemApi,
{
    let publisher = rosrust::publish(SYSTEM_HEALTH_SRV_NAME, QUEUE_SIZE)?;
    let task = async move {
        loop {
            let mut res = SystemHealthInfo::default();
            let health = api.system_health();
            res.peers = health.peers as u32;
            res.is_syncing = health.is_syncing;
            publisher.send(res).expect(SEND_MESSAGE_ERROR);
            Delay::new(PUBLISH_DELAY).await;
        }
    };
    Ok(task)
}

fn system_name<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: SystemApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(SYSTEM_NAME_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        res.success = true;
        res.message = api.system_name();
        Ok(res)
    })
}

fn system_version<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: SystemApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(SYSTEM_VERSION_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        res.success = true;
        res.message = api.system_version();
        Ok(res)
    })
}

fn system_chain<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: SystemApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(SYSTEM_CHAIN_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        res.success = true;
        res.message = api.system_chain();
        Ok(res)
    })
}

fn system_properties<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: SystemApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(SYSTEM_PROPERTIES_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        res.success = true;
        res.message = serde_json::to_string(&api.system_properties()).unwrap();
        Ok(res)
    })
}

fn system_health<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: SystemApi + Send + Sync + 'static,
{
    rosrust::service::<SystemHealth, _>(SYSTEM_HEALTH_SRV_NAME, move |_| {
        let mut res = SystemHealthRes::default();
        let health = api.system_health();
        res.info.peers = health.peers as u32;
        res.info.is_syncing = health.is_syncing;
        Ok(res)
    })
}

pub fn start_services<T>(api: T) -> Result<Vec<rosrust::Service>, Error>
where
    T: SystemApi + Clone + Send + Sync + 'static,
{
    let services = vec![
        system_name(api.clone())?,
        system_version(api.clone())?,
        system_chain(api.clone())?,
        system_properties(api.clone())?,
        system_health(api.clone())?,
    ];
    Ok(services)
}

pub fn start_publishers<T>(api: T) -> Result<impl Future<Output = ()>, Error>
where
    T: SystemApi + Clone,
{
    let task = futures::future::join4(
        publish_system_name(api.clone())?,
        publish_system_version(api.clone())?,
        publish_system_chain(api.clone())?,
        publish_system_health(api.clone())?,
    )
    .map(|_| ());
    Ok(task)
}
