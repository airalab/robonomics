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
use super::{
    AccountId, AllPalletsWithSystem, Balances, DealWithFees, MessageQueue, ParachainInfo,
    ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee,
    XcmpQueue,
};
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
    parameter_types,
    traits::{Contains, Disabled, Everything, Nothing, TransformOrigin},
    weights::Weight,
};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::traits::ConstU32;

// Polkadot imports
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowHrmpNotificationsFromRelayChain,
    AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
    DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin, FungibleAdapter, HashedDescription,
    IsConcrete, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
    WeightInfoBounds, WithComputedOrigin,
};
use xcm_executor::{traits::WithOriginFilter, Config, XcmExecutor};

pub const ASSET_HUB_ID: u32 = 1000;

parameter_types! {
    pub RelayNetwork: NetworkId = ParachainInfo::relay_network();
    pub UniversalLocation: InteriorLocation =
        [GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
    pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
    pub AssetHubParaId: ParaId = 1000.into();
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub SystemAssetHubLocation: Location = Location::new(1, [Parachain(ASSET_HUB_ID)]);
    pub AssetHubTrustedTeleporter: (AssetFilter, Location) = (NativeAssetFilter::get(), SystemAssetHubLocation::get());
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
    pub const NativeAssetId: AssetId = AssetId(Location::here());
    pub const NativeAssetFilter: AssetFilter = Wild(AllOf { fun: WildFungible, id: NativeAssetId::get() });
    pub const RelayLocation: Location = Location::parent();
    pub const LocalLocation: Location = Location::here();
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Foreign locations alias into accounts according to a hash of their standard description.
    HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<LocalLocation>,
    // Convert an XCM Location into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (FungibleTransactor,);

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
    pub UnitWeightCost: Weight = Weight::from_parts(1_000_000, 4 * 1024);
}

/// Match parent or plurality from parent.
pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
    fn contains(location: &Location) -> bool {
        if location.parents != 1 {
            return false;
        }
        match &location.interior {
            Here => true,
            Junctions::X1(arc) => {
                matches!(arc.as_ref(), [Junction::Plurality { .. }])
            }
            _ => false,
        }
    }
}

pub struct ParentOrParentsExecutivePlurality;
impl Contains<Location> for ParentOrParentsExecutivePlurality {
    fn contains(location: &Location) -> bool {
        matches!(
            location.unpack(),
            (1, [])
                | (
                    1,
                    [Plurality {
                        id: BodyId::Executive,
                        ..
                    }]
                )
        )
    }
}

pub type Barrier = TrailingSetTopicAsId<(
    TakeWeightCredit,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Allow XCMs with some computed origins to pass through.
    WithComputedOrigin<
        (
            // If the message is one that immediately attempts to pay for execution, then
            // allow it.
            AllowTopLevelPaidExecutionFrom<Everything>,
            // Parent and its pluralities (i.e. governance bodies) get free execution.
            AllowExplicitUnpaidExecutionFrom<(ParentOrParentsExecutivePlurality,)>,
            // Subscriptions for version tracking are OK.
            AllowSubscriptionsFrom<Everything>,
            // HRMP notifications from the relay chain are OK.
            AllowHrmpNotificationsFromRelayChain,
        ),
        UniversalLocation,
        ConstU32<8>,
    >,
)>;

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

/*
pub type WaivedLocations = (
    RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>,
);
*/

pub struct XcmConfig;
impl Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type XcmRecorder = PolkadotXcm;
    type XcmEventEmitter = PolkadotXcm;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = ();
    type IsTeleporter = xcm_builder::Case<AssetHubTrustedTeleporter>;
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::RobonomicsXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
    type Trader = UsingComponents<WeightToFee, LocalLocation, AccountId, Balances, DealWithFees>;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = xcm_executor::traits::WaiveDeliveryFees;
    /*
    type FeeManager = XcmFeeManagerFromComponents<
        WaivedLocations,
        SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>
    >;
    */
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<SafeCallFilter>;
    type SafeCallFilter = SafeCallFilter;
    type Aliasers = Nothing;
    type TransactionalProcessor = xcm_builder::FrameTransactionalProcessor;
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
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
    pub ReachableDest: Option<Location> = Some(Parent.into());
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
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::RobonomicsXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type AuthorizedAliasConsideration = Disabled;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
    pub const MaxInboundSuspended: u32 = 1000;
    pub const MaxActiveOutboundChannels: u32 = 128;
    pub const MaxPageSize: u32 = 65536;
    pub const BaseXcmpDeliveryFee: u128 = 1_000_000_000;
    pub const XcmpByteFee: u128 = 1_000_000;
}

/// Price for delivering an XCM to a sibling parachain destination.
pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
    RelayLocation,
    BaseXcmpDeliveryFee,
    XcmpByteFee,
    XcmpQueue,
>;

/// Convert a sibling `ParaId` to an `AggregateMessageOrigin`.
pub struct ParaIdToSibling;
impl sp_runtime::traits::Convert<ParaId, AggregateMessageOrigin> for ParaIdToSibling {
    fn convert(para_id: ParaId) -> AggregateMessageOrigin {
        AggregateMessageOrigin::Sibling(para_id)
    }
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = MaxInboundSuspended;
    type MaxActiveOutboundChannels = MaxActiveOutboundChannels;
    type MaxPageSize = MaxPageSize;
    type ControllerOrigin = frame_system::EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    // Charge a conservative fee for XCM message delivery to siblings
    type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
    type WeightInfo = crate::weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
}

impl cumulus_pallet_xcmp_queue::migration::v5::V5Config for Runtime {
    type ChannelList = ParachainSystem;
}
