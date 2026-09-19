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
//! Governance instances
//!
//! Implements a minimal permissionless OpenGov governance system controlled
//! by XRT holders, replacing privileged Sudo control:
//!
//! - a permissionless `Root` track for arbitrary privileged actions;
//! - a `Whitelisted Caller` fast track for time-sensitive technical
//!   proposals reviewed by the Core Team collective;
//! - no full Council or Fellowship: the Core Team collective is a small,
//!   `Root`-managed `pallet_collective` instance whose sole power is voting
//!   to whitelist call hashes for the fast track (see `WhitelistOrigin`
//!   below). It has no other privileges.

use super::*;
use core::cmp::Ordering;
use frame_support::traits::{
    fungible::HoldConsideration, EitherOf, EitherOfDiverse, Get, LinearStoragePrice, PrivilegeCmp,
};

mod origins;
pub use origins::{pallet_custom_origins, WhitelistedCaller};
mod tracks;
pub use tracks::TracksInfo;

/// The `pallet_collective` instance backing the Core Team collective.
pub type TechnicalCollective = pallet_collective::Instance1;

parameter_types! {
    // Preimage storage pricing is intentionally kept separate from the
    // generic `deposit()` helper used by proxy/multisig (see issue #629,
    // "Preimage economics"). These values are placeholders and MUST be
    // reviewed against realistic runtime WASM sizes before activation.
    pub const PreimageBaseDeposit: Balance = 100 * XRT;
    pub const PreimageByteDeposit: Balance = 10 * COASE;
    pub const PreimageHoldReason: RuntimeHoldReason =
        RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = HoldConsideration<
        AccountId,
        Balances,
        PreimageHoldReason,
        LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
    >;
    type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
}

/// Used to compare the privilege of an origin inside the scheduler.
///
/// `Root` is greater than anything else; no other origin is used for
/// `ScheduleOrigin` in this minimal governance model.
pub struct OriginPrivilegeCmp;
impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
    fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
        if left == right {
            return Some(Ordering::Equal);
        }

        match (left, right) {
            (OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type OriginPrivilegeCmp = OriginPrivilegeCmp;
    type Preimages = Preimage;
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
    type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
}

parameter_types! {
    pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

impl pallet_conviction_voting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type VoteLockingPeriod = VoteLockingPeriod;
    type MaxVotes = ConstU32<512>;
    type MaxTurnout =
        frame_support::traits::tokens::currency::ActiveIssuanceOf<Balances, AccountId>;
    type Polls = Referenda;
    type BlockNumberProvider = System;
    type VotingHooks = ();
    type WeightInfo = weights::pallet_conviction_voting::WeightInfo<Runtime>;
}

impl pallet_custom_origins::Config for Runtime {}

parameter_types! {
    // Time-boxed to roughly match the `whitelisted_caller` referenda track's
    // 3-day decision period, so committee review does not become the
    // bottleneck for the fast track it feeds.
    pub const TechCommMotionDuration: BlockNumber = 3 * DAYS;
    pub const TechCommMaxProposals: u32 = 20;
    pub const TechCommMaxMembers: u32 = 7;
    // Mirrors `MaximumSchedulerWeight` above: no single collective-dispatched
    // call may consume more than half a block's weight.
    pub MaxCollectiveProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

/// A small, `Root`-managed committee (see [`TechnicalCollective`])
/// whose only power is voting to authorize `WhitelistOrigin` below. It is
/// not a general-purpose Council or Technical Committee: `RuntimeCall`
/// dispatch via this collective is only ever reachable through
/// `pallet_whitelist`, whose `WhitelistOrigin`/`DispatchWhitelistedOrigin`
/// split keeps "who may whitelist a call" and "who may enact it" separate.
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = TechCommMotionDuration;
    type MaxProposals = TechCommMaxProposals;
    type MaxMembers = TechCommMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxCollectiveProposalWeight;
    type DisapproveOrigin = EnsureRoot<AccountId>;
    type KillOrigin = EnsureRoot<AccountId>;
    type Consideration = ();
}

impl pallet_whitelist::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    // At least half of the Core Team collective's members (not merely a
    // majority of those who bothered to vote) must approve, mirroring
    // `MoreThanHalfCouncil`-style thresholds used for technical committees
    // elsewhere in the ecosystem.
    type WhitelistOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 2>,
    >;
    type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<AccountId>, WhitelistedCaller>;
    type Preimages = Preimage;
    type WeightInfo = weights::pallet_whitelist::WeightInfo<Runtime>;
}

parameter_types! {
    pub const AlarmInterval: BlockNumber = 1;
    pub const SubmissionDeposit: Balance = 10 * XRT;
    pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
    pub const MaxQueued: u32 = 100;
}

impl pallet_referenda::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type Scheduler = Scheduler;
    type Currency = Balances;
    type SubmitOrigin = frame_system::EnsureSigned<AccountId>;
    type CancelOrigin = EnsureRoot<AccountId>;
    type KillOrigin = EnsureRoot<AccountId>;
    type Slash = ();
    type Votes = pallet_conviction_voting::VotesOf<Runtime>;
    type Tally = pallet_conviction_voting::TallyOf<Runtime>;
    type SubmissionDeposit = SubmissionDeposit;
    type MaxQueued = MaxQueued;
    type UndecidingTimeout = UndecidingTimeout;
    type AlarmInterval = AlarmInterval;
    type Tracks = TracksInfo;
    type Preimages = Preimage;
    type BlockNumberProvider = System;
    type WeightInfo = weights::pallet_referenda::WeightInfo<Runtime>;
}
