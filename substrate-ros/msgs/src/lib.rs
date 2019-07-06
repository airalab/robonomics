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
///! Rust generated ROS messages.
use rosrust::rosmsg_include;

rosmsg_include!(
    // standard ros messages
    std_msgs / UInt64,
    std_msgs / String,

    // robonomics messages
    substrate_ros_msgs / Demand,
    substrate_ros_msgs / Offer,
    substrate_ros_msgs / Finalize,
    substrate_ros_msgs / Liability,

    // substrate rpc messages
    substrate_ros_msgs / ExHash,
    substrate_ros_msgs / RawExtrinsic,

    substrate_ros_msgs / PendingExtrinsics,
    substrate_ros_msgs / RemoveExtrinsic,
    substrate_ros_msgs / SubmitExtrinsic,

    substrate_ros_msgs / BlockHash,
    substrate_ros_msgs / BlockNumber,
    substrate_ros_msgs / BlockHashByNumber,
    substrate_ros_msgs / BlockNumberByHash,
);
