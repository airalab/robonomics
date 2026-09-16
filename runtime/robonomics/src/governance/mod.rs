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
//!   proposals reviewed by the Robonomics core team;
//! - no Council, Technical Committee or Fellowship.

use super::*;
use core::cmp::Ordering;
use frame_support::traits::{fungible::HoldConsideration, EitherOf, LinearStoragePrice, PrivilegeCmp};

mod origins;
pub use origins::{pallet_custom_origins, WhitelistedCaller, WhitelistOrigin};
mod tracks;
pub use tracks::TracksInfo;

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
    type WeightInfo = ();
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
    type WeightInfo = ();
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
    type WeightInfo = ();
}

impl pallet_custom_origins::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_whitelist::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    // Root, or the core-team multisig, can whitelist a call hash. Neither
    // this alone grants `Root`: see `DispatchWhitelistedOrigin` below.
    type WhitelistOrigin = WhitelistOrigin;
    // A whitelisted call only ever dispatches as `Root` after the
    // `WhitelistedCaller` referendum has passed, or directly via `Root`.
    type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<AccountId>, WhitelistedCaller>;
    type Preimages = Preimage;
    type WeightInfo = ();
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
    // Any signed account can submit a Root referendum.
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
    type WeightInfo = ();
}
