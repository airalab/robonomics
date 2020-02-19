///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2019 Airalab <research@aira.life> 
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
//! Robonomics runtime traits definitions.

use frame_support::dispatch;
use sp_runtime::{DispatchResult, traits::Member};
use sp_arithmetic::traits::BaseArithmetic;

/// Technical aspects of agreement between two parties.
pub trait Technical {
    /// Technical parameter of agreement. Usually in this parameter one party of agreement
    /// provide technical information like task description, parameters and some additional data.
    type Parameter: dispatch::Parameter;

    /// During execution one part of agreement should prepare report,
    /// it's parameter of finalize transaction. Technically lifecycle of liability
    /// consist of three steps:
    ///  * Pre-open: parties try come to an agreement;
    ///  * Start: funds of one party locked, second party execute a task;
    ///  * Finish: funds of one party transfered as same as report of executed task published.
    type Report: dispatch::Parameter;

    /// Someone who can confirm task execution in real world.
    type Oracle: RealWorldOracle;
}

/// The arbiter of the real world.
pub trait RealWorldOracle {
    // TODO: @akru, design real world oracle interface.
}

/// Economical aspects of agreement between two parties.
pub trait Economical {
    /// Economical parameter of agreement. Usually in this parameter one party set task cost
    /// for another party. To come an agreement both parties should be agree with this parameter.
    type Parameter: dispatch::Parameter;
}

/// Transaction processing for economical aspects of agreement. Usually it consists of
/// balance locking and transfers when liability successfully finished.
pub trait Processing {
    /// This method called each time when liability started.
    fn on_start(&self) -> DispatchResult;

    /// This method called each time when liability finished.
    fn on_finish(&self, success: bool) -> DispatchResult;
}

/// Agreement between two participants around technical/economical aspects.
pub trait Agreement<T: Technical, E: Economical> {
    /// Indexing type.
    type Index: dispatch::Parameter + BaseArithmetic + Member + Copy + Default;

    /// Pariticipant account address.
    type AccountId: dispatch::Parameter;

    /// Some that could be used as proof of participants agreement.
    type Proof: dispatch::Parameter;

    /// Create new instance for given technical and economical parameters.
    fn new(
        technics:  T::Parameter,
        economics: E::Parameter,
        promisee:  Self::AccountId,
        promisor:  Self::AccountId,
    ) -> Self;

    /// Check validity of agreement params proof.
    fn check_params(
        &self,
        proof: &Self::Proof,
        sender: &Self::AccountId,
    ) -> bool;

    /// Check validity of agreement report proof.
    fn check_report(
        &self,
        index: &Self::Index,
        report: &T::Report,
        proof: &Self::Proof,
    ) -> bool;
}

/// Agreement proovement maker.
pub trait ProofBuilder<T: Technical, E: Economical, Index, Account, Proof> {
    /// Make proof of technical and economical agreement parameters.
    fn proof_params(
        technics: &T::Parameter,
        economics: &E::Parameter,
        sender: Account,
    ) -> Proof;

    /// Make proof of technical report agrement parameter.
    fn proof_report(
        index: &Index,
        report: &T::Report, 
        sender: Account,
    ) -> Proof;
}
