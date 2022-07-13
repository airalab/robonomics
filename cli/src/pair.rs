//! Robonomics I/O CLI interface.

#![deny(missing_docs)]

use crate::error::Result;

/// Substrate friendly CLI I/O subsystem interaction.
#[derive(structopt::StructOpt, Debug)]
pub struct PairCmd {
    /// I/O device operation to run.
    #[structopt(subcommand)]
    pub operation: Operation,
}

impl IoCmd {
    /// Run I/O operation on device.
    pub fn run(&self) -> Result<()> {
        match &self.operation {
           // Operation::Read(source) => source.run(),
           // Operation::Write(sink) => sink.run(),
        }
    }
}

/// I/O operation command.
#[derive(structopt::StructOpt, Debug)]
pub enum Operation {
    Read(),
    Write(),
    /// Read information from device.
    //Read(Cmd),
    /// Write information into device.
    //Write(super::SinkCmd),
}

