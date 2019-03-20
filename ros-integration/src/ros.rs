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

use std::sync::Mutex;
use rosrust::error::Result;
use rosrust::{
    api::{Ros, Topic, raii::{Publisher, Subscriber}},
    Message,
};

lazy_static! {
    static ref ROS: Mutex<Option<Ros>> = Mutex::new(None);
}

macro_rules! ros {
    () => {
        ROS.lock().unwrap().as_mut().unwrap()
    };
}

#[inline]
pub fn init() {
    let client = Some(Ros::new("robonomics").unwrap());
    let mut ros = ROS.lock().unwrap();
    *ros = client;
}

#[inline]
pub fn subscribe<T, F>(topic: &str, queue_size: usize, callback: F) -> Result<Subscriber>
where
    F: Fn(T) -> () + Send + 'static,
    T: Message,
{
    ros!().subscribe::<T, F>(topic, queue_size, callback)
}

#[inline]
pub fn publish<T>(topic: &str, queue_size: usize) -> Result<Publisher<T>>
where
    T: Message
{
    ros!().publish::<T>(topic, queue_size)
}

#[inline]
pub fn topics() -> Vec<Topic> {
    ros!().topics().unwrap()
}
