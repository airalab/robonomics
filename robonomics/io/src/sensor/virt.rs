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
///! Virtual sensors collection.
///
/// This module contains:
/// - Stdin: Standart input stream. 
///

use futures::channel::mpsc;
use crate::error::Result;
use std::io::BufRead;
use super::Sensor;
use std::thread;

/// Simple standart input.
pub struct Stdin;

impl Sensor for Stdin {
    type Config = ();
    type Measure = String;
    type Stream = mpsc::UnboundedReceiver<Self::Measure>;

    fn new(_config: Self::Config) -> Result<Self> {
        Ok(Stdin)
    }

    fn read(self) -> Self::Stream {
        let (tx, rx) = mpsc::unbounded();
        thread::spawn(move || {
            let input = std::io::stdin();
            for line in input.lock().lines() {
                let _ = tx.unbounded_send(line.expect("unable to read line from stdio"));
            }
        });
        rx
    }
}
