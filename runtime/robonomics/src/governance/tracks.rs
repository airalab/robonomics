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
//! Track configurations for the Robonomics governance.
//!
//! Only two tracks exist on purpose:
//!
//! - `root`, the general-purpose permissionless track used for arbitrary
//!   `Root` calls (runtime upgrades, protocol configuration, etc.);
//! - `whitelisted_caller`, the fast track for calls previously whitelisted
//!   by the core-team multisig.

use super::*;

use alloc::borrow::Cow;
use pallet_referenda::Curve;
use sp_runtime::{str_array as s, FixedI64};

const fn percent(x: i32) -> FixedI64 {
    FixedI64::from_rational(x as u128, 100)
}

// Root track: approval decays linearly from 90% to 51%, support from 50% to 15%,
// over the full 14-day decision period.
const APP_ROOT: Curve = Curve::make_linear(14, 14, percent(51), percent(90));
const SUP_ROOT: Curve = Curve::make_linear(14, 14, percent(15), percent(50));

// Whitelisted Caller track: approval decays from 99% to 80%, support from
// 2% down to 1% over the full 3-day decision period.
const APP_WHITELISTED_CALLER: Curve = Curve::make_linear(3, 3, percent(80), percent(99));
const SUP_WHITELISTED_CALLER: Curve = Curve::make_linear(3, 3, percent(1), percent(2));

const TRACKS_DATA: [pallet_referenda::Track<u16, Balance, BlockNumber>; 2] = [
    pallet_referenda::Track {
        id: 0,
        info: pallet_referenda::TrackInfo {
            name: s("root"),
            max_deciding: 1,
            decision_deposit: 5_000 * XRT,
            prepare_period: 12 * HOURS,
            decision_period: 14 * DAYS,
            confirm_period: 24 * HOURS,
            min_enactment_period: 1 * HOURS,
            min_approval: APP_ROOT,
            min_support: SUP_ROOT,
        },
    },
    pallet_referenda::Track {
        id: 1,
        info: pallet_referenda::TrackInfo {
            name: s("whitelisted_caller"),
            max_deciding: 10,
            decision_deposit: 1 * XRT,
            prepare_period: 5 * MINUTES,
            decision_period: 3 * DAYS,
            confirm_period: 20 * MINUTES,
            min_enactment_period: 5 * MINUTES,
            min_approval: APP_WHITELISTED_CALLER,
            min_support: SUP_WHITELISTED_CALLER,
        },
    },
];

/// Provides the two Robonomics governance tracks to `pallet-referenda`.
pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
    type Id = u16;
    type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;

    fn tracks(
    ) -> impl Iterator<Item = Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
    {
        TRACKS_DATA.iter().map(Cow::Borrowed)
    }

    fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
        if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
            match system_origin {
                frame_system::RawOrigin::Root => Ok(0),
                _ => Err(()),
            }
        } else if let Ok(custom_origin) = origins::Origin::try_from(id.clone()) {
            match custom_origin {
                origins::Origin::WhitelistedCaller => Ok(1),
            }
        } else {
            Err(())
        }
    }
}
