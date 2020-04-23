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
///! Robonomics sensing subsystem. 

use serde::Serialize;
use futures::Stream;
use crate::error::Result;

/// Collection of serial port sensors.
pub mod serial;

/// Collection of virtual sensors (like stdin).
pub mod virt;

/// Sensor is an hardware device that provide that cold provide some data of external world.
pub trait Sensor: Sized {
    /// Sensor initial parameters.
    type Config;

    /// Sensor data type.
    type Measure: Serialize + Send + Sync;

    /// Stream of measurements in the future.
    type Stream: Stream<Item = Self::Measure> + Sized;

    /// Create new sensor instance.
    fn new(config: Self::Config) -> Result<Self>;

    /// Read a data from sensor.
    /// Note: this method cannot be run twice.
    fn read(self) -> Self::Stream;
}
