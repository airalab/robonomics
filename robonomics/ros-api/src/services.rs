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
//! Robonomics ROS services implementation. 

use base58::FromBase58;

use msgs::robonomics_msgs::{
    SendOrder, SendOrderRes,
    SendReport, SendReportRes,
};
use crate::crypto::ExtrinsicSender;
use crate::error::Result;
use crate::error::Error;

pub fn send_demand<T: ExtrinsicSender>(sender: T) -> Result<rosrust::Service> {
    rosrust::service::<SendOrder, _>("liability/demand", move |req| {
        let mut res = SendOrderRes::default(); 
        let technics  = req.technics.from_base58();
        // TODO
        Ok(res)
    }).map_err(Error::RosError)
}

pub fn send_offer<T: ExtrinsicSender>(sender: T) -> Result<rosrust::Service> {
    rosrust::service::<SendOrder, _>("liability/offer", move |req| {
        let mut res = SendOrderRes::default(); 
        // TODO
        Ok(res)
    }).map_err(Error::RosError)
}

pub fn send_report<T: ExtrinsicSender>(sender: T) -> Result<rosrust::Service> {
    rosrust::service::<SendReport, _>("liability/report", move |req| {
        let mut res = SendReportRes::default(); 
        // TODO
        Ok(res)
    }).map_err(Error::RosError)
}
