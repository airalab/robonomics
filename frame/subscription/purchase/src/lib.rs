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
//! # Robonomics Subscription Purchase Allocator
//!
//! Sells a predefined subscription product. A product is a
//! protocol-configured template (price, validity duration, bounded resource
//! limits); users never choose limits themselves. A purchase builds the
//! normalized [`SubscriptionAllocation`] from the product and submits it to
//! the beneficiary through the generic [`SubscriptionAllocator`] interface.
//!
//! ## Responsibilities
//!
//! The pallet owns acquisition semantics: the catalogue and the price. The
//! subscription pallet knows neither and keeps no allocator identity, so a
//! purchase and a governance allocation of the same normalized allocation
//! produce identical subscription state. The pallet does not touch
//! subscription storage, does not depend on CPS and does not know how
//! transactions are metered.
//!
//! ## Payment: deposits
//!
//! The price is expressed in one external asset (for example USDT held by a
//! reserve chain) that never exists locally, and the pallet holds no asset:
//! it has no balances or credit. An inbound transfer handler of the runtime
//! calls [`Pallet::purchase_with_deposit`] with the received amount. A
//! deposit covering the price is burned as a whole (an excess is not
//! refunded) and the product is allocated to the account the deposit is made
//! to. If the deposit is below the price, the product is missing or disabled,
//! or the allocation fails, nothing happens and the caller rejects the
//! deposit, which stays with the sender.
//!
//! ## Invariants
//!
//! - Products reserve nothing. Capacity admission is performed by the
//!   subscription pallet when a purchase allocates, and its failure (for
//!   example `GlobalRateCapacityExceeded`) is propagated.
//! - A purchase is atomic and has no effect on failure.
//! - A purchase replaces the current subscription generation of the
//!   beneficiary. Unused time and resources never roll over.
//! - Changing or disabling a product affects future purchases only.

#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::frame_support;

#[cfg(test)]
mod tests;
pub mod types;
pub mod weights;

pub use pallet::*;
pub use types::{ProductId, SubscriptionProduct};
pub use weights::WeightInfo;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use super::*;
    use polkadot_sdk::frame_support::{
        pallet_prelude::*, storage::with_storage_layer, traits::Get,
    };
    use polkadot_sdk::frame_system::{self, pallet_prelude::*};
    use robonomics_subscription_primitives::{ResourceAllocation, SubscriptionAllocator};

    type AllocatorOf<T> = <T as Config>::Allocator;
    type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    /// Resource identifier of the configured allocator.
    pub type ResourceIdOf<T> =
        <AllocatorOf<T> as SubscriptionAllocator<AccountIdOf<T>>>::ResourceId;
    /// Resource limit of the configured allocator.
    pub type LimitOf<T> = <AllocatorOf<T> as SubscriptionAllocator<AccountIdOf<T>>>::Limit;
    /// Clock unit of the configured allocator.
    pub type MomentOf<T> = <AllocatorOf<T> as SubscriptionAllocator<AccountIdOf<T>>>::Moment;
    /// Resource bound of the configured allocator.
    pub type MaxResourcesOf<T> =
        <AllocatorOf<T> as SubscriptionAllocator<AccountIdOf<T>>>::MaxResources;
    /// Resource entitlements of a product.
    pub type ResourcesOf<T> =
        BoundedVec<ResourceAllocation<ResourceIdOf<T>, LimitOf<T>>, MaxResourcesOf<T>>;
    /// Product stored by the pallet.
    pub type ProductOf<T> = SubscriptionProduct<
        <T as Config>::Balance,
        MomentOf<T>,
        ResourceIdOf<T>,
        LimitOf<T>,
        MaxResourcesOf<T>,
    >;

    #[pallet::config]
    pub trait Config: polkadot_sdk::frame_system::Config {
        /// The overarching event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

        /// Origin allowed to create, update and disable products.
        type ManagementOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Subscription allocation backend.
        type Allocator: SubscriptionAllocator<
            Self::AccountId,
            ResourceId: Parameter + MaxEncodedLen,
            Limit: Parameter + MaxEncodedLen,
            Moment: Parameter + MaxEncodedLen,
            MaxResources: 'static,
        >;

        /// Amount of the payment asset, used for prices.
        type Balance: Parameter + MaxEncodedLen + PartialOrd;

        /// Product bought by deposits, see [`Pallet::purchase_with_deposit`].
        #[pallet::constant]
        type DepositProduct: Get<ProductId>;

        /// Weight information.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Product catalogue.
    #[pallet::storage]
    pub type Products<T: Config> = StorageMap<_, Blake2_128Concat, ProductId, ProductOf<T>>;

    /// Identifier the next created product gets. Identifiers are never reused.
    #[pallet::storage]
    pub type NextProductId<T: Config> = StorageValue<_, ProductId, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A product was created.
        ProductCreated { product_id: ProductId },
        /// A product was updated.
        ProductUpdated { product_id: ProductId },
        /// A product was disabled.
        ProductDisabled { product_id: ProductId },
        /// A product was purchased with a deposit and its allocation granted.
        Purchased {
            beneficiary: T::AccountId,
            product_id: ProductId,
            paid: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The product does not exist.
        UnknownProduct,
        /// The product is disabled and cannot be purchased.
        ProductDisabled,
        /// Product identifiers are exhausted.
        ProductIdOverflow,
        /// The deposit does not cover the price of the product.
        DepositBelowPrice,
    }

    impl<T: Config> Pallet<T> {
        /// Buy [`Config::DepositProduct`] for `beneficiary` with a deposit of
        /// `amount`.
        ///
        /// Meant to be called by the inbound transfer handler of the runtime,
        /// which then burns the whole deposit: the pallet holds nothing and an
        /// excess over the price is not refunded. Fails without effect when the
        /// deposit is below the price, the product is missing or disabled, or
        /// the allocation fails (for example `GlobalRateCapacityExceeded`), so
        /// the caller can reject the deposit.
        pub fn purchase_with_deposit(
            beneficiary: &T::AccountId,
            amount: T::Balance,
        ) -> DispatchResult {
            with_storage_layer(|| {
                let product_id = T::DepositProduct::get();
                let product = Products::<T>::get(product_id).ok_or(Error::<T>::UnknownProduct)?;
                ensure!(product.enabled, Error::<T>::ProductDisabled);
                ensure!(amount >= product.price, Error::<T>::DepositBelowPrice);

                T::Allocator::allocate(beneficiary, product.allocation())?;

                Self::deposit_event(Event::Purchased {
                    beneficiary: beneficiary.clone(),
                    product_id,
                    paid: amount,
                });
                Ok(())
            })
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create an enabled product.
        ///
        /// The product is not validated against the subscription pallet and
        /// reserves no capacity; an invalid template fails when purchased.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_product(resources.len() as u32))]
        pub fn create_product(
            origin: OriginFor<T>,
            price: T::Balance,
            valid_for: MomentOf<T>,
            resources: ResourcesOf<T>,
        ) -> DispatchResult {
            T::ManagementOrigin::ensure_origin(origin)?;
            let product_id = NextProductId::<T>::get();
            let next = product_id
                .checked_add(1)
                .ok_or(Error::<T>::ProductIdOverflow)?;
            NextProductId::<T>::put(next);
            Products::<T>::insert(
                product_id,
                SubscriptionProduct {
                    price,
                    valid_for,
                    resources,
                    enabled: true,
                },
            );
            Self::deposit_event(Event::ProductCreated { product_id });
            Ok(())
        }

        /// Replace the template of a product, keeping it enabled or disabled.
        ///
        /// Affects future purchases only; existing subscriptions are
        /// unchanged.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_product(resources.len() as u32))]
        pub fn update_product(
            origin: OriginFor<T>,
            product_id: ProductId,
            price: T::Balance,
            valid_for: MomentOf<T>,
            resources: ResourcesOf<T>,
        ) -> DispatchResult {
            T::ManagementOrigin::ensure_origin(origin)?;
            Products::<T>::try_mutate(product_id, |product| -> DispatchResult {
                let product = product.as_mut().ok_or(Error::<T>::UnknownProduct)?;
                product.price = price;
                product.valid_for = valid_for;
                product.resources = resources;
                Ok(())
            })?;
            Self::deposit_event(Event::ProductUpdated { product_id });
            Ok(())
        }

        /// Disable a product so it can no longer be purchased.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::disable_product())]
        pub fn disable_product(origin: OriginFor<T>, product_id: ProductId) -> DispatchResult {
            T::ManagementOrigin::ensure_origin(origin)?;
            Products::<T>::try_mutate(product_id, |product| -> DispatchResult {
                product.as_mut().ok_or(Error::<T>::UnknownProduct)?.enabled = false;
                Ok(())
            })?;
            Self::deposit_event(Event::ProductDisabled { product_id });
            Ok(())
        }
    }
}
