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
///! Serial port sensors collection.
///
/// This module contains:
/// - SDS011: Nova laser particle sensor.
///

use futures::channel::mpsc;
use crate::error::Result;
use std::time::Duration;
use super::Sensor;
use std::thread;

/// Nova SDS011 particle sensor.
pub struct SDS011 {
    device: sds011::SDS011,
    work_period_secs: u64,
}

/// SDS011 sensor configuration.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SDS011Config {
    /// Serial port that connected sensor
    /// For example: `/dev/ttyUSB0` or `COM11`
    pub port: String,
    /// Working period in minutes, must be in interval (0..30)
    pub period: u8,
}

impl Sensor for SDS011 {
    type Config = SDS011Config;
    type Measure = sds011::Message;
    type Stream = mpsc::UnboundedReceiver<Self::Measure>;

    fn new(config: Self::Config) -> Result<Self> {
        let mut device = sds011::SDS011::new(config.port.as_str())?;
        device.set_work_period(config.period)?;
        log::debug!(
            target: "robonomics-sensors",
            "SDS011: sensor created for {:?}", config
        );
        let work_period_secs = config.period as u64 * 60;
        Ok(SDS011 { device, work_period_secs })
    }

    fn read(self) -> Self::Stream {
        let (sender, receiver) = mpsc::unbounded();
        thread::spawn(move || sds011_worker(self, sender));
        receiver
    }
}

fn sds011_worker(
    mut sensor: SDS011,
    result: mpsc::UnboundedSender<sds011::Message>,
) {
    let delay = Duration::from_secs(sensor.work_period_secs);
    loop {
        if let Some(message) = sensor.device.query() {
            log::debug!(
                target: "robonomics-sensors",
                "SDS011: data read {:?}", message
            );
            let _ = result.unbounded_send(message);
        }
        thread::sleep(delay);
    }
}
