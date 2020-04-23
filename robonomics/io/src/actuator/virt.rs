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
///! Virtual actuators collection.
///
/// This module contains:
/// - Stdout: Standart output stream. 
///

use futures::{future, Stream, Future, StreamExt};
use std::io::{self, Write};
use crate::error::Result;
use super::Actuator;

/// Simple standart output.
pub struct Stdout;

impl Actuator for Stdout {
    type Config = ();
    type Control = String;

    fn new(_config: Self::Config) -> Result<Self> {
        Ok(Stdout)
    }

    fn write<'a, T: Stream<Item = Self::Control> + 'a>(
        self,
        control: T, 
    ) -> Box<dyn Future<Output = ()> + 'a> {
        Box::new(control.for_each(|msg| {
            io::stdout()
                .write_all(msg.as_bytes())
                .expect("unable to write string");
            future::ready(())
        }))
    }
}
