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
//! # Robonomics XCM Teleport Pallet
//!
//! This pallet provides a simplified send functionality for Robonomics parachain
//! to send assets to Asset Hub parachain using XCM.
//!
//! ## Overview
//!
//! The pallet implements a strict version of XCM teleport with the following constraints:
//! - Only supports sending the native asset (pallet_balances)
//! - Only supports sending to Asset Hub parachain
//! - Uses relay chain asset for fees on Asset Hub
//! - Beneficiary specified as AccountId32 (32-byte account ID)
//!
//! The send process follows this pattern:
//! 1. Build XCM message with WithdrawAsset and InitiateTransfer instructions
//! 2. InitiateTransfer executes locally to withdraw assets and sends to destination
//! 3. On Asset Hub: PayFees (with relay asset) → DepositAsset to beneficiary

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::vec;
    use xcm::prelude::*;
    use xcm::opaque::latest::AssetTransferFilter;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for balance operations
        type Currency: Currency<Self::AccountId>;

        /// XCM message sender
        type XcmSender: SendXcm;

        /// Asset Hub Location
        #[pallet::constant]
        type AssetHubLocation: Get<Location>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Assets have been sent to Asset Hub. [origin, beneficiary, asset]
        Sent {
            origin: T::AccountId,
            beneficiary: Location,
            asset: Asset,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Failed to send XCM message
        SendFailure,
        /// Failed to execute XCM locally
        LocalExecutionFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Send native assets to Asset Hub parachain.
        ///
        /// This extrinsic sends native assets (XRT) from the caller's account to a beneficiary
        /// account on the Asset Hub parachain. The transfer uses XCM InitiateTransfer to:
        /// 1. Initiate transfer to Asset Hub with teleport semantics
        /// 2. Pay fees on Asset Hub using relay chain asset
        /// 3. Deposit assets to beneficiary on Asset Hub
        ///
        /// # Parameters
        /// - `origin`: The account sending the assets
        /// - `beneficiary`: The recipient AccountId32 (32-byte account ID) on Asset Hub
        /// - `amount`: The amount of native asset to send
        /// - `fee`: The amount of relay chain asset to use for fees on Asset Hub
        ///
        /// # Errors
        /// - `SendFailure`: Failed to send XCM message
        #[pallet::call_index(0)]
        #[pallet::weight({
            // Simple weight based on XCM execution
            Weight::from_parts(100_000_000, 10_000)
        })]
        pub fn send(
            origin: OriginFor<T>,
            beneficiary: [u8; 32],
            amount: BalanceOf<T>,
            fee: u128,
        ) -> DispatchResult {
            let origin_account = ensure_signed(origin)?;

            // Build beneficiary location from AccountId32
            let beneficiary_location: Location = AccountId32 {
                network: None,
                id: beneficiary,
            }
            .into();

            // Destination is always Asset Hub
            let dest = T::AssetHubLocation::get();

            // Convert amount to u128 for XCM (u128 is the widest type available)
            let xcm_amount: u128 = amount.try_into().unwrap_or(u128::MAX);

            // Build the native asset
            let native_asset = Asset {
                id: AssetId(Location::here()),
                fun: Fungibility::Fungible(xcm_amount),
            };

            let assets: Assets = vec![native_asset.clone()].into();

            // Build the relay asset for fees (on Asset Hub, fees are paid in relay chain asset)
            let relay_fee_asset = Asset {
                id: AssetId(Location::parent()),
                fun: Fungibility::Fungible(fee),
            };

            // Build the XCM message using InitiateTransfer for teleport
            // InitiateTransfer will execute locally and send to destination
            let message: Xcm<()> = Xcm(vec![
                WithdrawAsset(assets.clone()),
                InitiateTransfer {
                    destination: dest.clone(),
                    remote_fees: Some(AssetTransferFilter::Teleport(Wild(AllCounted(1)))),
                    preserve_origin: false,
                    assets: vec![AssetTransferFilter::Teleport(Wild(AllCounted(1)))]
                        .try_into()
                        .unwrap_or_default(),
                    remote_xcm: Xcm(vec![
                        PayFees {
                            asset: relay_fee_asset,
                        },
                        DepositAsset {
                            assets: Wild(AllCounted(1)),
                            beneficiary: beneficiary_location.clone(),
                        },
                    ]),
                },
            ]);

            // Send the message using XcmSender
            let (ticket, _) = T::XcmSender::validate(&mut Some(dest.clone()), &mut Some(message))
                .map_err(|_| Error::<T>::SendFailure)?;
            T::XcmSender::deliver(ticket)
                .map_err(|_| Error::<T>::SendFailure)?;

            // Emit event
            Self::deposit_event(Event::Sent {
                origin: origin_account,
                beneficiary: beneficiary_location,
                asset: native_asset,
            });

            Ok(())
        }
    }
}
