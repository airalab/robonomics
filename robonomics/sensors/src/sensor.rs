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
use crate::error::Result;
use std::thread::sleep;
use std::time::Duration;
use sds011::SDS011;

pub async fn read_loop(port: &str, work_period: u8) -> Result<()> {
    match SDS011::new(port) {
        Ok(mut sensor) => {
            sensor.set_work_period(work_period)?;

            loop {
                if let Some(m) = sensor.query() {
                    //let mut wtr = Writer::from_writer(vec![]);
                    //wtr.serialize(m)?;
                    //let show = String::from_utf8(wtr.into_inner()?)?;
                    log::info!(target: "robonomics-sensors", "{:?}", m);
                }

                sleep(Duration::from_secs(work_period as u64 * 60));
            }
        },
        Err(e) => log::error!(
            target: "robonomics-sensors", "{:?}", e.description),
    };
    Ok(())
}