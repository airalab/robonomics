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
//! Baseline transaction usage.

use crate::{BalanceOf, Config, Pallet};
use polkadot_sdk::{
    frame_support::{
        dispatch::{DispatchClass, DispatchInfo, PostDispatchInfo},
        pallet_prelude::Pays,
        traits::Get,
        weights::Weight,
    },
    frame_system, pallet_transaction_payment,
    sp_runtime::traits::Saturating,
};

impl<T: Config> Pallet<T> {
    /// Baseline cost of a transaction in the native currency:
    ///
    /// ```text
    /// base fee + length fee + weight fee
    /// ```
    ///
    /// It reuses the fee primitives of `pallet-transaction-payment` but
    /// leaves out the congestion multiplier and the tip, so the same call,
    /// length and weight cost the same under any network load. Currency fee
    /// exemptions do not waive Subscription resource usage.
    pub fn baseline_cost(
        len: u32,
        weight: Weight,
        _pays_fee: Pays,
        class: DispatchClass,
    ) -> BalanceOf<T> {
        let base_fee = pallet_transaction_payment::Pallet::<T>::weight_to_fee(
            <T as frame_system::Config>::BlockWeights::get()
                .get(class)
                .base_extrinsic,
        );
        let len_fee = pallet_transaction_payment::Pallet::<T>::length_to_fee(len);
        let weight_fee = pallet_transaction_payment::Pallet::<T>::weight_to_fee(weight);
        base_fee.saturating_add(len_fee).saturating_add(weight_fee)
    }

    /// Baseline cost estimated before dispatch from the declared weight.
    pub fn estimated_cost(len: usize, info: &DispatchInfo) -> BalanceOf<T> {
        Self::baseline_cost(len as u32, info.total_weight(), info.pays_fee, info.class)
    }

    /// Baseline cost after dispatch from the actual weight.
    pub fn actual_cost(
        len: usize,
        info: &DispatchInfo,
        post_info: &PostDispatchInfo,
    ) -> BalanceOf<T> {
        Self::baseline_cost(
            len as u32,
            post_info.calc_actual_weight(info),
            post_info.pays_fee(info),
            info.class,
        )
    }
}
