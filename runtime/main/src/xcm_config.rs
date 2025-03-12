///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2024 Robonomics Network <research@robonomics.network>
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
use super::{
    AccountId, AllPalletsWithSystem, AssetId, Assets, Balance, Balances, DealWithFees,
    ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
    WeightToFee, XcmInfo, XcmpQueue,
};
use frame_support::{
    match_types,
    pallet_prelude::Get,
    parameter_types,
    traits::{Contains, ContainsPair, Everything, Nothing, PalletInfoAccess},
    weights::Weight,
};
use sp_runtime::traits::ConstU32;
use sp_std::{marker::PhantomData, prelude::*};

// Polkadot imports
use xcm::latest::prelude::*;
use xcm_builder::{
    Account32Hash, AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteId, EnsureXcmOrigin,
    FixedRateOfFungible, FixedWeightBounds, FungibleAdapter, FungiblesAdapter, IsConcrete,
    NoChecking, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{
    traits::{JustTry, WithOriginFilter},
    Config, XcmExecutor,
};

parameter_types! {
    pub RelayNetwork: NetworkId = XcmInfo::relay_network().unwrap_or(NetworkId::Kusama);
    pub const RelayLocation: MultiLocation = MultiLocation::parent();
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorMultiLocation =
        X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
    pub Local: MultiLocation = Here.into_location();
    pub AssetsPalletLocation: MultiLocation =
        PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
    pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<polkadot_parachain::primitives::Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Derives a private `Account32` by hashing `("multiloc", received multilocation)`
    Account32Hash<RelayNetwork, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = FungibleAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<Local>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteId<AssetId, Balance, XcmInfo, JustTry>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Assets`.
    NoChecking,
    // We don't track any teleports of `Assets`.
    DummyCheckingAccount,
>;

/// Means for transacting the native currency on this chain.
pub type BridgedCurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<EthereumCurrencyLocation>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports.
    (),
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (
    CurrencyTransactor,
    BridgedCurrencyTransactor,
    FungiblesTransactor,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    pallet_xcm::XcmPassthrough<RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);

parameter_types! {
    // One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
    pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 4 * 1024);
    pub const MaxInstructions: u32 = 100;
    pub KsmPerSecond: (cumulus_primitives_core::AssetId, u128, u128) =
        (MultiLocation::parent().into(), 1_000_000_000, 1_000_000_000);
}

match_types! {
    pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
    };
}

pub type XcmBarrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    // Parent and its plurality get free execution
    AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Subscriptions for version tracking are OK.
    AllowSubscriptionsFrom<Everything>,
);

/// Asset filter that allows all assets from a certain location.
pub struct AssetsFrom<T>(PhantomData<T>);
impl<T: Get<MultiLocation>> ContainsPair<MultiAsset, MultiLocation> for AssetsFrom<T> {
    fn contains(_a: &MultiAsset, b: &MultiLocation) -> bool {
        b.eq(&T::get())
    }
}

/// A call filter for the XCM Transact instruction.
pub struct SafeCallFilter;
impl SafeCallFilter {
    pub fn allow_base_call(_call: &RuntimeCall) -> bool {
        // Allow almost base calls by default
        true
    }
    pub fn allow_composite_call(call: &RuntimeCall) -> bool {
        match call {
            /*
            RuntimeCall::Proxy(pallet_proxy::Call::proxy { call, .. }) => {
                Self::allow_base_call(call)
            }
            RuntimeCall::Proxy(pallet_proxy::Call::proxy_announced { call, .. }) => {
                Self::allow_base_call(call)
            }
            */
            RuntimeCall::Utility(pallet_utility::Call::batch { calls, .. }) => {
                calls.iter().all(|call| Self::allow_base_call(call))
            }
            RuntimeCall::Utility(pallet_utility::Call::batch_all { calls, .. }) => {
                calls.iter().all(|call| Self::allow_base_call(call))
            }
            RuntimeCall::Utility(pallet_utility::Call::as_derivative { call, .. }) => {
                Self::allow_base_call(call)
            }
            RuntimeCall::Multisig(pallet_multisig::Call::as_multi_threshold_1 { call, .. }) => {
                Self::allow_base_call(call)
            }
            RuntimeCall::Multisig(pallet_multisig::Call::as_multi { call, .. }) => {
                Self::allow_base_call(call)
            }
            _ => false,
        }
    }
}

impl Contains<RuntimeCall> for SafeCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        Self::allow_base_call(call) || Self::allow_composite_call(call)
    }
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = AssetsFrom<RelayLocation>;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = XcmBarrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = (
        FixedRateOfFungible<KsmPerSecond, ()>,
        UsingComponents<WeightToFee, Local, AccountId, Balances, DealWithFees>,
    );
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<SafeCallFilter>;
    type SafeCallFilter = SafeCallFilter;
    type Aliasers = Nothing;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;

    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = pallet_xcm::TestWeightInfo; // TODO: fix weights
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
    type ControllerOrigin = frame_system::EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type PriceForSiblingDelivery = ();
    type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl pallet_xcm_info::Config for Runtime {
    type AssetId = AssetId;
    type RuntimeEvent = RuntimeEvent;
}
