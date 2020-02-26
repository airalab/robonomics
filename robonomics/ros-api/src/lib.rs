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
//! This module exports Robonomics API into ROS namespace.

pub mod agent;
pub mod messages;
pub mod services;

use crate as robonomics_ros_api;

#[macro_export]
macro_rules! start {
    ($client:expr) => {{
        robonomics_ros_api::agent::print_account($client.clone());
        (
            robonomics_ros_api::messages::receive_stream($client.clone())?,
            (
                robonomics_ros_api::services::send_demand($client.clone())?,
                robonomics_ros_api::services::send_offer($client.clone())?,
                robonomics_ros_api::services::send_report($client.clone())?,
                robonomics_ros_api::services::send_record($client.clone())?,
            )
        )
    }}
}
