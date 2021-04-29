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

use msgs::std_srvs::{Trigger, TriggerRes};
use msgs::substrate_ros_msgs::{
    ExHash, PendingExtrinsics, PendingExtrinsicsRes, RawExtrinsic, RemoveExtrinsic,
    RemoveExtrinsicRes, SubmitExtrinsic, SubmitExtrinsicRes,
};
use rosrust::api::error::Error;

const SUBMIT_SRV_NAME: &str = "/author/submit_extrinsic";
const REMOVE_SRV_NAME: &str = "/author/remove_extrinsic";
const PENDING_SRV_NAME: &str = "/author/pending_extrinsics";
const ROTATE_KEYS_SRV_NAME: &str = "/author/rotate_keys";

pub type Bytes = Vec<u8>;
pub type Hash = [u8; 32];

/// Substrate Author API that exported to ROS namespace.
pub trait AuthorApi {
    /// Generate new session keys and returns the corresponding public keys.
    fn rotate_keys(&self) -> Result<Bytes, String>;

    /// Submit hex-encoded extrinsic for inclusion in block.
    fn submit_extrinsic(&self, ext: Bytes) -> Result<Hash, String>;

    /// Returns all pending extrinsics, potentially grouped by sender.
    fn pending_extrinsics(&self) -> Vec<Bytes>;

    /// Remove given extrinsic from the pool and temporarily ban it to prevent reimporting.
    fn remove_extrinsics(&self, hashes: Vec<Hash>) -> Vec<Hash>;
}

fn rotate_keys<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: AuthorApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(ROTATE_KEYS_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        let keys = api.rotate_keys()?;
        res.success = true;
        res.message = format!("{}", hex::encode(&keys));
        Ok(res)
    })
}

fn submit_extrinsic<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: AuthorApi + Send + Sync + 'static,
{
    rosrust::service::<SubmitExtrinsic, _>(SUBMIT_SRV_NAME, move |req| {
        let mut res = SubmitExtrinsicRes::default();
        res.hash = ExHash::default();
        res.hash.data = api.submit_extrinsic(req.extrinsic.data)?;
        Ok(res)
    })
}

fn pending_extrinsics<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: AuthorApi + Send + Sync + 'static,
{
    rosrust::service::<PendingExtrinsics, _>(PENDING_SRV_NAME, move |_req| {
        let mut res = PendingExtrinsicsRes::default();
        for xt in api.pending_extrinsics() {
            let mut xt_msg = RawExtrinsic::default();
            xt_msg.data.extend(xt);
            res.extrinsics.push(xt_msg);
        }
        Ok(res)
    })
}

fn remove_extrinsics<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: AuthorApi + Send + Sync + 'static,
{
    rosrust::service::<RemoveExtrinsic, _>(REMOVE_SRV_NAME, move |req| {
        let mut res = RemoveExtrinsicRes::default();
        let hashes = req.extrinsics.iter().map(|h| h.data).collect();
        for xt in api.remove_extrinsics(hashes) {
            let mut hash_msg = ExHash::default();
            hash_msg.data = xt;
            res.extrinsics.push(hash_msg);
        }
        Ok(res)
    })
}

pub fn start_services<T>(api: T) -> Result<Vec<rosrust::Service>, Error>
where
    T: AuthorApi + Clone + Send + Sync + 'static,
{
    let services = vec![
        rotate_keys(api.clone())?,
        submit_extrinsic(api.clone())?,
        pending_extrinsics(api.clone())?,
        remove_extrinsics(api.clone())?,
    ];
    Ok(services)
}
