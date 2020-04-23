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
///! Robonomics action subsystem.

use futures::{Future, Stream};
use crate::error::Result;

/// Collection of virtual actuators (like stdout).
pub mod virt;

/// An actuator is a component of a machine that is responsible for moving and controlling a
/// mechanism or system.
pub trait Actuator: Sized {
    /// Actuator initial parameters.
    type Config;

    /// An actuator control.
    type Action;

    /// Actuator control stream.
    type Control: Stream<Item = Self::Action> + Sized;

    /// Actuator execution task.
    type Task: Future<Output = ()> + Sized;

    /// Create new actuator instance.
    fn new(config: Self::Config) -> Result<Self>;

    /// Control an actuator by control stream.
    /// Note: this method cannot be run twice.
    fn write(self, control: Self::Control) -> Self::Task;
}
