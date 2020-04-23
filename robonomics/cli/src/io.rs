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
///! Robonomics I/O CLI interface.

use crate::error::Result;

/// Substrate friendly CLI I/O subsystem interaction command.
#[derive(structopt::StructOpt, Clone, Debug)]
pub struct IoCmd {
    #[structopt(subcommand)]
    pub operation: Operation,
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: sc_cli::SharedParams,
}

impl sc_cli::CliConfiguration for IoCmd {
	fn shared_params(&self) -> &sc_cli::SharedParams {
		&self.shared_params
	}
}

impl IoCmd {
    pub fn run(&self) -> Result<()> {
        match &self.operation {
            Operation::In(sensor) => sensor.run(),
            Operation::Out(actuator) => actuator.run(),
            Operation::Proc(processor) => processor.run(),
        }
    }
}

/// I/O operation command.
#[derive(structopt::StructOpt, Clone, Debug)]
pub enum Operation {
    In(super::SensorCmd),
    Out(super::ActuatorCmd),
    Proc(super::ProcessorCmd),
}
