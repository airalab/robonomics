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

use futures::{Stream, StreamExt, channel::mpsc};
use crate::error::Result;
use std::time::Duration;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::thread;

/// Nova SDS011 particle sensor.
pub struct SDS011(Pin<Box<dyn Stream<Item = sds011::Message> + Send>>);

impl SDS011 {
    /// Returns Nova SDS011 sensor instance.
    ///
    /// # Arguments
    /// * `port` - Serial port that connected sensor, for example: `/dev/ttyUSB0` or `COM11`
    /// * `period` - Working period in minutes, must be in interval (0..30)
    ///
    pub fn new(
        port: String,
        period: u8,
    ) -> Result<Self> {
        let mut device = sds011::SDS011::new(port.as_str())?;
        device.set_work_period(period)?;
        log::debug!(
            target: "robonomics-io",
            "SDS011: created for port {} with period {} min", port, period
        );
        let work_period_secs = period as u64 * 60;
        let (sender, receiver) = mpsc::unbounded();
        thread::spawn(move || sds011_worker(device, work_period_secs, sender));

        Ok(Self(receiver.boxed()))
    }
}

impl Stream for SDS011 {
    type Item = sds011::Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

fn sds011_worker(
    mut device: sds011::SDS011,
    work_period_secs: u64,
    result: mpsc::UnboundedSender<sds011::Message>,
) {
    let delay = Duration::from_secs(work_period_secs);
    loop {
        if let Some(message) = device.query() {
            log::debug!(
                target: "robonomics-io",
                "SDS011: data read {:?}", message
            );
            let _ = result.unbounded_send(message);
        }
        thread::sleep(delay);
    }
}
