///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2026 Robonomics Network <research@robonomics.network>
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
//! Robonomics liability traits definitions.

use frame_support::dispatch;
use sp_runtime::DispatchResult;

/// Transaction processing of agreement. Usually it consists of
/// balance locking and transfers when liability successfully finished.
pub trait Processing {
    /// This method called each time when liability started.
    fn on_start(&self) -> DispatchResult;

    /// This method called each time when liability finished.
    fn on_finish(&self, success: bool) -> DispatchResult;
}

/// Someone who can confirm agreement execution in real world.
pub trait RealWorldOracle {
    /// Waiting for replay from real world oracle.
    ///   None -> oracle decision isn't ready yet;
    ///   Some(true) -> oracle agree with report;
    ///   Some(false) -> oracle disagree with the report.
    fn is_confirmed(&self) -> Option<bool>;
}

/// During execution one part of agreement should prepare report,
/// it's parameter of finalize transaction. Technically, lifecycle of liability
/// consist of three steps:
///  * Pre-open: parties try come to an agreement;
///  * Start: funds of one party locked, second party execute a task;
///  * Finish: funds of one party transfered as same as report of executed task published.
pub trait Report<Index, AccountId>: RealWorldOracle {
    /// The report payload.
    type Message: dispatch::Parameter;

    /// Agreement indexing type.
    fn index(&self) -> Index;

    /// Sender account.
    fn sender(&self) -> AccountId;

    /// Check validity report proof.
    fn verify(&self) -> bool;
}

/// Agreement between two participants around technical/economical aspects.
pub trait Agreement<AccountId> {
    /// Technical parameter of agreement. Usually in this parameter one party of agreement
    /// provide technical information like task description, parameters and some additional data.
    type Technical: dispatch::Parameter;

    /// Economical parameter of agreement. Usually in this parameter one party set task cost
    /// for another party. To come an agreement both parties should be agree with this parameter.
    type Economical: dispatch::Parameter;

    /// Get techincal parameter of agreement.
    fn technical(&self) -> Self::Technical;

    /// Get Economical parameter of agreement.
    fn economical(&self) -> Self::Economical;

    /// The client account.
    fn promisee(&self) -> AccountId;

    /// The executive account.
    fn promisor(&self) -> AccountId;

    /// Check validity of agreement params proof.
    fn verify(&self) -> bool;
}

/// Agreement proof maker.
pub trait AgreementProofBuilder<Technical, Economical, Account, Proof> {
    /// Make proof of technical and economical agreement parameters.
    fn proof(technics: &Technical, economics: &Economical, sender: &Account) -> Proof;
}

/// Report proof maker.
pub trait ReportProofBuilder<Index, Message, Account, Proof> {
    /// Make proof of technical report agrement parameter.
    fn proof(index: &Index, message: &Message, sender: &Account) -> Proof;
}
