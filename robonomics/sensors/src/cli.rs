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
use robonomics_protocol::error::Result;
use async_std::task;
use crate::sensor;

/// Command for sensor
#[derive(Debug, structopt::StructOpt, Clone)]
pub struct SensorCmd {
    /// Sensor serial port
    #[structopt(long, default_value = "/dev/ttyUSB0")]
    port: String,

    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub shared_params: sc_cli::SharedParams,
    #[allow(missing_docs)]
    #[structopt(flatten)]
    pub import_params: sc_cli::ImportParams,
}

impl sc_cli::CliConfiguration for SensorCmd {
    fn shared_params(&self) -> &sc_cli::SharedParams {
        &self.shared_params
    }

    fn import_params(&self) -> Option<&sc_cli::ImportParams> {
        Some(&self.import_params)
    }
}

impl SensorCmd {
    /// Runs the command and node as sensor reader
    pub fn run(&self) -> Result<()> {
        task::block_on(sensor::read_loop(self.port.to_string().as_ref()))
    }
}