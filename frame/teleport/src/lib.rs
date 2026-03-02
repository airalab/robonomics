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
//! This pallet provides a simplified teleport functionality for Robonomics parachain
//! to send assets to Asset Hub parachain using XCM.
//!
//! ## Overview
//!
//! The pallet implements a strict version of XCM teleport with the following constraints:
//! - Only supports teleporting the native asset (pallet_balances)
//! - Only supports teleporting to Asset Hub parachain
//! - Uses parachain balance on Asset Hub for fees
//!
//! The teleport process follows this pattern:
//! 1. Build XCM message with WithdrawAsset, InitiateTeleport, and DepositAsset instructions
//! 2. Execute the message locally to withdraw assets
//! 3. Send the message to Asset Hub destination

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::Encode;
    use sp_runtime::traits::BlakeTwo256;
    use sp_std::boxed::Box;
    use xcm::prelude::*;
    use xcm_executor::traits::ConvertLocation;

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

        /// XCM executor
        type XcmExecutor: ExecuteXcm<<Self as frame_system::Config>::RuntimeCall>;

        /// Convert XCM Location to AccountId
        type LocationToAccountId: ConvertLocation<Self::AccountId>;

        /// Asset Hub parachain ID
        #[pallet::constant]
        type AssetHubParaId: Get<u32>;

        /// The pallet ID, used for deriving sovereign account
        #[pallet::constant]
        type PalletId: Get<frame_support::PalletId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Assets have been teleported. [origin, destination, beneficiary, amount]
        AssetsTeleported {
            origin: T::AccountId,
            destination: Location,
            beneficiary: Location,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Failed to send XCM message
        SendFailure,
        /// Failed to execute XCM locally
        LocalExecutionFailed,
        /// Invalid beneficiary
        InvalidBeneficiary,
        /// Invalid asset
        InvalidAsset,
        /// Unsupported XCM version
        UnsupportedVersion,
        /// Insufficient balance for teleport
        InsufficientBalance,
        /// Amount must be greater than zero
        ZeroAmount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Teleport native assets to Asset Hub parachain.
        ///
        /// This extrinsic teleports native assets (XRT) from the caller's account to a beneficiary
        /// account on the Asset Hub parachain. The teleport uses XCM to:
        /// 1. Withdraw assets from the caller
        /// 2. Initiate teleport to Asset Hub
        /// 3. Deposit assets to beneficiary on Asset Hub
        ///
        /// # Parameters
        /// - `origin`: The account teleporting the assets
        /// - `beneficiary`: The recipient account on Asset Hub
        /// - `amount`: The amount of native asset to teleport
        ///
        /// # Errors
        /// - `ZeroAmount`: Amount to teleport is zero
        /// - `InsufficientBalance`: Caller doesn't have enough balance
        /// - `LocalExecutionFailed`: Failed to execute XCM locally
        /// - `SendFailure`: Failed to send XCM message
        #[pallet::call_index(0)]
        #[pallet::weight({
            // Simple weight based on XCM execution
            Weight::from_parts(100_000_000, 10_000)
        })]
        pub fn teleport_assets(
            origin: OriginFor<T>,
            beneficiary: Box<VersionedLocation>,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let origin_account = ensure_signed(origin)?;

            // Convert versioned beneficiary to latest version
            let beneficiary: Location = (*beneficiary)
                .try_into()
                .map_err(|_| Error::<T>::UnsupportedVersion)?;

            // Ensure amount is not zero
            ensure!(amount > BalanceOf::<T>::from(0u32), Error::<T>::ZeroAmount);

            // Check if sender has sufficient balance
            ensure!(
                T::Currency::free_balance(&origin_account) >= amount,
                Error::<T>::InsufficientBalance
            );

            // Destination is always Asset Hub
            let asset_hub_para_id = T::AssetHubParaId::get();
            let dest = Location::new(1, [Parachain(asset_hub_para_id)]);

            // Convert amount to u128 for XCM
            let xcm_amount: u128 = amount
                .try_into()
                .map_err(|_| Error::<T>::InvalidAsset)?;

            // Build the native asset
            let asset = Asset {
                id: AssetId(Location::here()),
                fun: Fungibility::Fungible(xcm_amount),
            };

            let assets: Assets = vec![asset.clone()].into();

            // Build the XCM message following the teleport pattern
            // The message will:
            // 1. WithdrawAsset from holding register
            // 2. InitiateTeleport to start the teleport
            // 3. On destination, BuyExecution and DepositAsset to beneficiary
            let message: Xcm<<T as frame_system::Config>::RuntimeCall> = Xcm(vec![
                WithdrawAsset(assets.clone()),
                InitiateTeleport {
                    assets: Wild(AllCounted(1)),
                    dest: dest.clone(),
                    xcm: Xcm(vec![
                        BuyExecution {
                            fees: asset,
                            weight_limit: Unlimited,
                        },
                        DepositAsset {
                            assets: Wild(AllCounted(1)),
                            beneficiary: beneficiary.clone(),
                        },
                    ]),
                },
            ]);

            // Execute the message locally to withdraw assets from the origin account
            let origin_location: Location = AccountId32 {
                network: None,
                id: origin_account.encode().try_into().unwrap_or([0u8; 32]),
            }
            .into();

            let mut hash = message.using_encoded(|encoded| {
                use sp_runtime::traits::Hash as HashT;
                BlakeTwo256::hash(encoded).into()
            });

            let outcome = T::XcmExecutor::prepare_and_execute(
                origin_location,
                message,
                &mut hash,
                Weight::from_parts(1_000_000_000, 100_000),
                Weight::zero(),
            );

            // Check if local execution was successful
            ensure!(
                outcome.ensure_complete().is_ok(),
                Error::<T>::LocalExecutionFailed
            );

            // Emit event
            Self::deposit_event(Event::AssetsTeleported {
                origin: origin_account,
                destination: dest,
                beneficiary,
                amount,
            });

            Ok(())
        }
    }
}
