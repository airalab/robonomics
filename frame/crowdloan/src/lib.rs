///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2021 Robonomics Network <research@robonomics.network>
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
//! Robonomics crowdloan runtime module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use sp_std::prelude::*;
use xcm::prelude::*;

/// Prepare relay transaction XCM.
fn transact_xcm(call: xcm::DoubleEncoded<()>, para_id: u32) -> Xcm<()> {
    let fees = MultiAsset {
        id: AssetId::Concrete(MultiLocation::here()),
        fun: Fungibility::Fungible(1_000_000_000_000), // 1 KSM should be enough for any call
    };

    Xcm(vec![
        WithdrawAsset(fees.clone().into()),
        BuyExecution {
            fees,
            weight_limit: WeightLimit::Unlimited,
        },
        Transact {
            origin_type: OriginKind::SovereignAccount,
            require_weight_at_most: 20_000_000_000, // should be enough for any call
            call,
        },
        RefundSurplus,
        DepositAsset {
            assets: Wild(All),
            max_assets: 1,
            beneficiary: Parachain(para_id).into(),
        },
    ])
}

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;
    use polkadot_primitives::v2::{HeadData, Id as ParaId, ValidationCode};
    use polkadot_runtime_common::{crowdloan, paras_registrar as registrar, traits::Auctioneer};

    use super::*;

    type BalanceOf<T> = <<<T as crowdloan::Config>::Auctioneer as Auctioneer<
        <T as frame_system::Config>::BlockNumber,
    >>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config + TypeInfo {
        /// Local parachain ID.
        type ParachainId: Get<ParaId>;
        /// The type used to actually dispatch an XCM to its destination.
        type XcmRouter: SendXcm;
        /// The relay chain configuration type.
        type RelayRuntime: registrar::Config + crowdloan::Config;
        /// The relay chain call type, used for making correct XCM transaction.
        type RelayCall: From<registrar::Call<Self::RelayRuntime>>
            + From<crowdloan::Call<Self::RelayRuntime>>
            + Encode;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Fail to send XCM message
        XcmSendFailure,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Successful send XCM message
        XcmSent,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register new parachain ID using XCM.
        #[pallet::weight(100_000_000)]
        pub fn reserve(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            // get local parachain ID
            let my_id = T::ParachainId::get();

            // create valid runtime call
            // this call should register para_id and create parathread
            let call: T::RelayCall = registrar::Call::reserve {}.into();

            // create XVM call for the parachain
            let xcm = transact_xcm(call.encode().into(), my_id.into());
            let dest = MultiLocation::parent();

            // send XCM message using local router
            T::XcmRouter::send_xcm(dest, xcm).map_err(|_| Error::<T>::XcmSendFailure)?;

            Self::deposit_event(Event::XcmSent);
            Ok(().into())
        }

        /// Upload head and validation code for parachain ID using XCM.
        #[pallet::weight(100_000_000)]
        pub fn upload_head_code(
            origin: OriginFor<T>,
            #[pallet::compact] id: ParaId,
            genesis_head: HeadData,
            validation_code: ValidationCode,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            // get local parachain ID
            let my_id = T::ParachainId::get();

            // create valid runtime call
            // this call should register para_id and create parathread
            let call: T::RelayCall = registrar::Call::register {
                id,
                genesis_head,
                validation_code,
            }
            .into();

            // create XVM call for the parachain
            let xcm = transact_xcm(call.encode().into(), my_id.into());
            let dest = MultiLocation::parent();

            // send XCM message using local router
            T::XcmRouter::send_xcm(dest, xcm).map_err(|_| Error::<T>::XcmSendFailure)?;

            Self::deposit_event(Event::XcmSent);
            Ok(().into())
        }

        /// Create new crowdloan using XCM.
        #[pallet::weight(100_000_000)]
        pub fn start(
            origin: OriginFor<T>,
            #[pallet::compact] index: ParaId,
            #[pallet::compact] cap: BalanceOf<T::RelayRuntime>,
            #[pallet::compact] first_period: BlockNumberFor<T::RelayRuntime>,
            #[pallet::compact] last_period: BlockNumberFor<T::RelayRuntime>,
            #[pallet::compact] end: BlockNumberFor<T::RelayRuntime>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            // get local parachain ID
            let my_id = T::ParachainId::get();

            // create valid runtime call
            // this call should start new crowdloan for owned parachain id
            let call: T::RelayCall = crowdloan::Call::create {
                index,
                cap,
                first_period,
                last_period,
                end,
                verifier: None,
            }
            .into();

            // create XVM call for the parachain
            let xcm = transact_xcm(call.encode().into(), my_id.into());
            let dest = MultiLocation::parent();

            // send XCM message using local router
            T::XcmRouter::send_xcm(dest, xcm).map_err(|_| Error::<T>::XcmSendFailure)?;

            Self::deposit_event(Event::XcmSent);
            Ok(().into())
        }

        /// Swap parachain leases using XCM.
        #[pallet::weight(100_000_000)]
        pub fn swap(
            origin: OriginFor<T>,
            #[pallet::compact] target: ParaId,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            // get local parachain ID
            let my_id = T::ParachainId::get();

            // create valid runtime call
            // this call should swap para_id and other, both should be owned
            let call: T::RelayCall = registrar::Call::swap {
                id: my_id.clone(),
                other: target,
            }
            .into();

            // create XVM call for the parachain
            let xcm = transact_xcm(call.encode().into(), my_id.into());
            let dest = MultiLocation::parent();

            // send XCM message using local router
            T::XcmRouter::send_xcm(dest, xcm).map_err(|_| Error::<T>::XcmSendFailure)?;

            Self::deposit_event(Event::XcmSent);
            Ok(().into())
        }
    }
}
