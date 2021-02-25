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
    StateCall, StateCallRes, StorageHash, StorageHashRes, StorageKey, StorageKeys, StorageKeysRes,
    StorageQuery, StorageQueryRes, StorageSize, StorageSizeRes,
};
use rosrust::api::error::Error;

const CALL_SRV_NAME: &str = "/state/call";
const KEYS_SRV_NAME: &str = "/state/keys";
const QUERY_SRV_NAME: &str = "/state/query";
const HASH_SRV_NAME: &str = "/state/hash";
const SIZE_SRV_NAME: &str = "/state/size";
const VERSION_SRV_NAME: &str = "/runtime/version";

pub type Hash = [u8; 32];
pub type Bytes = Vec<u8>;

pub trait StateApi {
    /// Call a module at a block's state.
    fn call(&self, method: String, data: Bytes, block: Option<Hash>) -> Result<Bytes, String>;

    /// Returns the keys with prefix, leave empty to get all the keys
    fn storage_keys(&self, key_prefix: Bytes, block: Option<Hash>) -> Result<Vec<Bytes>, String>;

    /// Returns a storage entry at a specific block's state.
    fn storage(&self, key: Bytes, block: Option<Hash>) -> Result<Option<Bytes>, String>;

    /// Returns the hash of a storage entry at a block's state.
    fn storage_hash(&self, key: Bytes, block: Option<Hash>) -> Result<Option<Hash>, String>;

    /// Returns the size of a storage entry at a block's state.
    fn storage_size(&self, key: Bytes, block: Option<Hash>) -> Result<Option<u64>, String>;

    /// Get the runtime version.
    fn runtime_version(&self, hash: Option<Hash>) -> Result<String, String>;
}

fn zero_guard(mb_zero: Hash) -> Option<Hash> {
    if mb_zero == [0u8; 32] {
        None
    } else {
        Some(mb_zero)
    }
}

fn call<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<StateCall, _>(CALL_SRV_NAME, move |req| {
        let mut res = StateCallRes::default();
        res.data = api.call(req.method, req.data, zero_guard(req.block.data))?;
        Ok(res)
    })
}

fn storage_keys<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<StorageKeys, _>(KEYS_SRV_NAME, move |req| {
        let mut res = StorageKeysRes::default();
        for key in api.storage_keys(req.prefix.data, zero_guard(req.block.data))? {
            res.keys.push(StorageKey { data: key });
        }
        Ok(res)
    })
}

fn storage<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<StorageQuery, _>(QUERY_SRV_NAME, move |req| {
        let mut res = StorageQueryRes::default();
        if let Some(data) = api.storage(req.key.data, zero_guard(req.block.data))? {
            res.data = data;
        }
        Ok(res)
    })
}

fn storage_hash<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<StorageHash, _>(HASH_SRV_NAME, move |req| {
        let mut res = StorageHashRes::default();
        if let Some(hash) = api.storage_hash(req.key.data, zero_guard(req.block.data))? {
            res.hash.data = hash;
        }
        Ok(res)
    })
}

fn storage_size<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<StorageSize, _>(SIZE_SRV_NAME, move |req| {
        let mut res = StorageSizeRes::default();
        if let Some(size) = api.storage_size(req.key.data, zero_guard(req.block.data))? {
            res.size = size;
        }
        Ok(res)
    })
}

fn runtime_version<T>(api: T) -> Result<rosrust::Service, Error>
where
    T: StateApi + Send + Sync + 'static,
{
    rosrust::service::<Trigger, _>(VERSION_SRV_NAME, move |_| {
        let mut res = TriggerRes::default();
        res.success = true;
        res.message = api.runtime_version(None)?;
        Ok(res)
    })
}

pub fn start_services<T>(api: T) -> Result<Vec<rosrust::Service>, Error>
where
    T: StateApi + Clone + Send + Sync + 'static,
{
    let services = vec![
        call(api.clone())?,
        storage_keys(api.clone())?,
        storage(api.clone())?,
        storage_hash(api.clone())?,
        storage_size(api.clone())?,
        runtime_version(api.clone())?,
    ];
    Ok(services)
}
