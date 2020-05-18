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
//! Serial port sensors collection.

use async_std::task;
use futures::channel::mpsc;
use futures::prelude::*;
use futures_timer::Delay;
use std::time::Duration;

use crate::error::Result;

/// Nova SDS011 particle sensor.
///
/// # Arguments
/// * `port` - Serial port that connected sensor, for example: `/dev/ttyUSB0` or `COM11`
/// * `period` - Working period in minutes, must be in interval (0..30)
///
/// Returns Nova SDS011 sensor instance.
pub fn sds011(port: String, period: u8) -> Result<impl Stream<Item = Result<sds011::Message>>> {
    let mut device = sds011::SDS011::new(port.as_str())?;
    device.set_work_period(period)?;
    log::debug!(
        target: "robonomics-io",
        "SDS011: created for port {} with period {} min", port, period
    );

    let delay = Duration::from_secs(period as u64 * 60);
    let (sender, receiver) = mpsc::unbounded();
    task::spawn(async move {
        loop {
            let _ = sender.unbounded_send(device.query().map_err(Into::into));
            Delay::new(delay).await;
        }
    });

    Ok(receiver)
}
