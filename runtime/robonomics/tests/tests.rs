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
#![cfg(test)]

use polkadot_sdk::sp_core::crypto::Ss58Codec;
use polkadot_sdk::staging_xcm::latest::prelude::*;
use polkadot_sdk::xcm_runtime_apis::conversions::LocationToAccountHelper;
use robonomics_runtime::{xcm_config::LocationToAccountId, AccountId};

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
    // the purpose of hardcoded values is to catch an unintended location conversion logic change.
    struct TestCase {
        description: &'static str,
        location: Location,
        expected_account_id_str: &'static str,
    }

    let test_cases = vec![
        // DescribeTerminus
        TestCase {
            description: "DescribeTerminus Parent",
            location: Location::new(1, Here),
            expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
        },
        TestCase {
            description: "DescribeTerminus Sibling",
            location: Location::new(1, [Parachain(1111)]),
            expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
        },
        // DescribePalletTerminal
        TestCase {
            description: "DescribePalletTerminal Parent",
            location: Location::new(1, [PalletInstance(50)]),
            expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
        },
        TestCase {
            description: "DescribePalletTerminal Sibling",
            location: Location::new(1, [Parachain(1111), PalletInstance(50)]),
            expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
        },
        // DescribeAccountId32Terminal
        TestCase {
            description: "DescribeAccountId32Terminal Parent",
            location: Location::new(
                1,
                [Junction::AccountId32 {
                    network: None,
                    id: AccountId::from(ALICE).into(),
                }],
            ),
            expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
        },
        TestCase {
            description: "DescribeAccountId32Terminal Sibling",
            location: Location::new(
                1,
                [
                    Parachain(1111),
                    Junction::AccountId32 {
                        network: None,
                        id: AccountId::from(ALICE).into(),
                    },
                ],
            ),
            expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
        },
        // DescribeAccountKey20Terminal
        TestCase {
            description: "DescribeAccountKey20Terminal Parent",
            location: Location::new(
                1,
                [AccountKey20 {
                    network: None,
                    key: [0u8; 20],
                }],
            ),
            expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
        },
        TestCase {
            description: "DescribeAccountKey20Terminal Sibling",
            location: Location::new(
                1,
                [
                    Parachain(1111),
                    AccountKey20 {
                        network: None,
                        key: [0u8; 20],
                    },
                ],
            ),
            expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
        },
        // DescribeTreasuryVoiceTerminal
        TestCase {
            description: "DescribeTreasuryVoiceTerminal Parent",
            location: Location::new(
                1,
                [Plurality {
                    id: BodyId::Treasury,
                    part: BodyPart::Voice,
                }],
            ),
            expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
        },
        TestCase {
            description: "DescribeTreasuryVoiceTerminal Sibling",
            location: Location::new(
                1,
                [
                    Parachain(1111),
                    Plurality {
                        id: BodyId::Treasury,
                        part: BodyPart::Voice,
                    },
                ],
            ),
            expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
        },
        // DescribeBodyTerminal
        TestCase {
            description: "DescribeBodyTerminal Parent",
            location: Location::new(
                1,
                [Plurality {
                    id: BodyId::Unit,
                    part: BodyPart::Voice,
                }],
            ),
            expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
        },
        TestCase {
            description: "DescribeBodyTerminal Sibling",
            location: Location::new(
                1,
                [
                    Parachain(1111),
                    Plurality {
                        id: BodyId::Unit,
                        part: BodyPart::Voice,
                    },
                ],
            ),
            expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
        },
    ];

    for tc in test_cases {
        let expected =
            AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

        let got = LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
            tc.location.into(),
        )
        .unwrap();

        assert_eq!(got, expected, "{}", tc.description);
    }
}

mod cps_view_functions {
    use pallet_robonomics_cps::{
        pallet::{
            HasCapabilityViewFunction, IdentityOfViewFunction, NodeByIdentityViewFunction,
            ResolveScopeViewFunction,
        },
        Capability, Error, IdentityNode, NodeId, NodeIdentity, ResolvedScope,
    };
    use parity_scale_codec::{Decode, Encode};
    use polkadot_sdk::frame_support::view_functions::{
        ViewFunction, ViewFunctionDispatchError, ViewFunctionId,
    };
    use polkadot_sdk::sp_io;
    use robonomics_runtime::{AccountId, Runtime};

    #[test]
    fn runtime_executes_cps_view_functions() {
        sp_io::TestExternalities::default().execute_with(|| {
            let call = ResolveScopeViewFunction::<Runtime>::new(NodeId(0));
            let output = Runtime::execute_view_function(
                ResolveScopeViewFunction::<Runtime>::id(),
                call.encode(),
            )
            .expect("view function is dispatched by the runtime");
            let result =
                Result::<ResolvedScope<AccountId>, Error<Runtime>>::decode(&mut &output[..])
                    .expect("output decodes");
            assert_eq!(result, Err(Error::<Runtime>::NodeNotFound));

            let call = HasCapabilityViewFunction::<Runtime>::new(
                NodeId(0),
                AccountId::from([1u8; 32]),
                Capability::Write,
            );
            let output = Runtime::execute_view_function(
                HasCapabilityViewFunction::<Runtime>::id(),
                call.encode(),
            )
            .expect("view function is dispatched by the runtime");
            let result =
                Result::<bool, Error<Runtime>>::decode(&mut &output[..]).expect("output decodes");
            assert_eq!(result, Err(Error::<Runtime>::NodeNotFound));
        });
    }

    #[test]
    fn runtime_executes_cps_identity_view_functions() {
        sp_io::TestExternalities::default().execute_with(|| {
            let account = AccountId::from([1u8; 32]);
            let node = NodeId(7);
            NodeIdentity::<Runtime>::insert(node, &account);
            IdentityNode::<Runtime>::insert(&account, node);

            let call = IdentityOfViewFunction::<Runtime>::new(node);
            let output = Runtime::execute_view_function(
                IdentityOfViewFunction::<Runtime>::id(),
                call.encode(),
            )
            .expect("view function is dispatched by the runtime");
            assert_eq!(
                Option::<AccountId>::decode(&mut &output[..]).expect("output decodes"),
                Some(account.clone())
            );

            let call = NodeByIdentityViewFunction::<Runtime>::new(account);
            let output = Runtime::execute_view_function(
                NodeByIdentityViewFunction::<Runtime>::id(),
                call.encode(),
            )
            .expect("view function is dispatched by the runtime");
            assert_eq!(
                Option::<NodeId>::decode(&mut &output[..]).expect("output decodes"),
                Some(node)
            );
        });
    }

    #[test]
    fn runtime_rejects_unknown_view_function() {
        sp_io::TestExternalities::default().execute_with(|| {
            let mut id: ViewFunctionId = ResolveScopeViewFunction::<Runtime>::id();
            id.suffix = [0xff; 16];
            assert!(matches!(
                Runtime::execute_view_function(id, Vec::new()),
                Err(ViewFunctionDispatchError::NotFound(_))
            ));
        });
    }
}

mod subscription_purchase_xcm {
    use frame_support::{assert_ok, BoundedVec};
    use pallet_robonomics_subscription::{Limit, Subscriptions};
    use robonomics_runtime::{
        xcm_config::{AssetHubLocation, LocationToAccountId, UsdtId, XcmConfig},
        AccountId, Purchase, Runtime, RuntimeCall, RuntimeOrigin,
    };
    use robonomics_subscription_primitives::{Resource, ResourceAllocation};
    use xcm::latest::prelude::*;
    use xcm_executor::{traits::ConvertLocation, XcmExecutor};

    const USER: [u8; 32] = [7u8; 32];
    const PRICE: u128 = 1_000_000;
    const FEE_BUDGET: u128 = PRICE / 10;

    /// The user as seen from this chain: an account of AssetHub.
    fn user_location() -> Location {
        Location::new(
            1,
            [
                Parachain(1000),
                AccountId32 {
                    network: None,
                    id: USER,
                },
            ],
        )
    }

    fn user_account() -> AccountId {
        LocationToAccountId::convert_location(&user_location()).expect("location converts")
    }

    fn usdt(amount: u128) -> Asset {
        (UsdtId::get(), amount).into()
    }

    fn create_product(rate: u128) {
        assert_ok!(Purchase::create_product(
            RuntimeOrigin::root(),
            PRICE,
            60_000,
            BoundedVec::truncate_from(vec![ResourceAllocation {
                resource: Resource::Transaction,
                limit: Limit::Transaction(rate),
            }]),
        ));
    }

    /// Message AssetHub sends to deposit `deposit` USDT to the user.
    fn deposit_message(deposit: u128) -> Xcm<RuntimeCall> {
        Xcm(vec![
            ReserveAssetDeposited(usdt(deposit).into()),
            BuyExecution {
                fees: usdt(FEE_BUDGET),
                weight_limit: Unlimited,
            },
            DepositAsset {
                assets: Wild(AllCounted(1)),
                beneficiary: user_location(),
            },
        ])
    }

    fn execute(origin: Location, message: Xcm<RuntimeCall>) -> Outcome {
        let mut id = [0u8; 32];
        XcmExecutor::<XcmConfig>::prepare_and_execute(
            origin,
            message,
            &mut id,
            Weight::from_parts(10_000_000_000, 1_000_000),
            Weight::zero(),
        )
    }

    #[test]
    fn deposit_from_asset_hub_buys_the_product() {
        sp_io::TestExternalities::default().execute_with(|| {
            create_product(1_000);
            let outcome = execute(AssetHubLocation::get(), deposit_message(2 * PRICE));
            assert!(matches!(outcome, Outcome::Complete { .. }), "{outcome:?}");

            assert!(Subscriptions::<Runtime>::get(user_account()).is_some());
        });
    }

    #[test]
    fn deposit_below_the_price_is_rejected() {
        sp_io::TestExternalities::default().execute_with(|| {
            create_product(1_000);
            let outcome = execute(AssetHubLocation::get(), deposit_message(PRICE - 1));
            assert!(matches!(outcome, Outcome::Incomplete { .. }), "{outcome:?}");

            assert!(Subscriptions::<Runtime>::get(user_account()).is_none());
        });
    }

    #[test]
    fn deposit_is_rejected_when_the_allocation_fails() {
        sp_io::TestExternalities::default().execute_with(|| {
            // Far above the capacity reserved for subscriptions.
            create_product(u128::MAX / 2);
            let outcome = execute(AssetHubLocation::get(), deposit_message(2 * PRICE));
            assert!(matches!(outcome, Outcome::Incomplete { .. }), "{outcome:?}");

            assert!(Subscriptions::<Runtime>::get(user_account()).is_none());
        });
    }

    #[test]
    fn deposit_is_rejected_from_other_chains() {
        sp_io::TestExternalities::default().execute_with(|| {
            create_product(1_000);
            let sibling = Location::new(1, [Parachain(2000)]);
            let outcome = execute(sibling, deposit_message(2 * PRICE));
            assert!(matches!(outcome, Outcome::Incomplete { .. }), "{outcome:?}");

            assert!(Subscriptions::<Runtime>::get(user_account()).is_none());
        });
    }
}

mod subscription_transaction {
    use frame_support::{
        assert_ok,
        dispatch::{DispatchInfo, GetDispatchInfo},
        traits::fungible::{Inspect, Mutate},
        BoundedVec,
    };
    use frame_system::RawOrigin;
    use pallet_robonomics_cps::{Capability, GrantMode, NodeId};
    use pallet_robonomics_subscription::Limit;
    use pallet_robonomics_transaction::{priority, ChargeTransaction};
    use robonomics_runtime::{
        AccountId, Balances, Runtime, RuntimeCall, Subscription, Transaction, TxExtension, CPS, XRT,
    };
    use robonomics_subscription_primitives::{
        AuthorizationContext, Resource, ResourceAllocation, SubscriptionAllocation,
        SubscriptionAllocator,
    };
    use sp_runtime::{
        traits::{DispatchTransaction, TransactionExtension},
        transaction_validity::{InvalidTransaction, TransactionSource},
    };

    const LEN: usize = 100;

    fn account(id: u8) -> AccountId {
        AccountId::new([id; 32])
    }

    fn call() -> RuntimeCall {
        RuntimeCall::System(frame_system::Call::remark { remark: vec![] })
    }

    fn info() -> DispatchInfo {
        call().get_dispatch_info()
    }

    fn set_time(now: u64) {
        pallet_timestamp::Now::<Runtime>::put(now);
    }

    /// Subscribe `owner` with the whole network Transaction capacity and let
    /// the bucket fill.
    fn subscribe(owner: &AccountId) -> u64 {
        set_time(1_000);
        assert_ok!(Subscription::allocate(
            owner,
            SubscriptionAllocation {
                valid_for: 10_000_000,
                resources: BoundedVec::truncate_from(vec![ResourceAllocation {
                    resource: Resource::Transaction,
                    limit: Limit::Transaction(XRT / 1_000),
                }]),
            }
        ));
        set_time(1_000 + 6_000);
        pallet_robonomics_subscription::Subscriptions::<Runtime>::get(owner)
            .unwrap()
            .generation
    }

    #[test]
    fn runtime_routes_payment_through_charge_transaction() {
        let identifiers: Vec<_> = <TxExtension as TransactionExtension<RuntimeCall>>::metadata()
            .into_iter()
            .map(|extension| extension.identifier)
            .collect();
        assert!(identifiers.contains(&"ChargeTransaction"));
        assert!(!identifiers.contains(&"ChargeTransactionPayment"));
        assert_eq!(robonomics_runtime::VERSION.transaction_version, 5);
    }

    #[test]
    fn fee_mode_keeps_ordinary_payment() {
        sp_io::TestExternalities::default().execute_with(|| {
            let who = account(1);
            assert_ok!(Balances::mint_into(&who, 1_000 * XRT));
            let before = Balances::balance(&who);
            let expected =
                pallet_transaction_payment::Pallet::<Runtime>::compute_fee(LEN as u32, &info(), 0);

            let (_, origin) = ChargeTransaction::<Runtime>::fee(0)
                .validate_and_prepare(
                    RawOrigin::Signed(who.clone()).into(),
                    &call(),
                    &info(),
                    LEN,
                    0,
                )
                .expect("fee payment works");
            assert!(expected > 0);
            assert_eq!(Balances::balance(&who), before - expected);
            assert_eq!(frame_system::ensure_signed(origin).ok(), Some(who));
        });
    }

    #[test]
    fn account_subscription_pays_without_fees_and_below_ordinary_fee_class() {
        sp_io::TestExternalities::default().execute_with(|| {
            let who = account(1);
            assert_ok!(Balances::mint_into(&who, 1_000 * XRT));
            let generation = subscribe(&who);
            let balance = Balances::balance(&who);
            let ext = || {
                ChargeTransaction::<Runtime>::subscription(
                    AuthorizationContext::Account(who.clone()),
                    generation,
                )
            };

            let (valid, _, _) = ext()
                .validate_only(
                    RawOrigin::Signed(who.clone()).into(),
                    &call(),
                    &info(),
                    LEN,
                    TransactionSource::External,
                    0,
                )
                .expect("subscription transaction is valid");
            let fee_priority = ChargeTransaction::<Runtime>::fee(500 * XRT)
                .validate_only(
                    RawOrigin::Signed(who.clone()).into(),
                    &call(),
                    &info(),
                    LEN,
                    TransactionSource::External,
                    0,
                )
                .expect("fee transaction is valid")
                .0
                .priority;
            assert!(valid.priority >= priority::SUBSCRIPTION_BASE);
            assert!(fee_priority < priority::SUBSCRIPTION_BASE);

            assert_ok!(ext().validate_and_prepare(
                RawOrigin::Signed(who.clone()).into(),
                &call(),
                &info(),
                LEN,
                0
            ));
            assert_eq!(Balances::balance(&who), balance);
            let cost = Transaction::estimated_cost(LEN, &info());
            assert!(cost > 0 && cost < 6 * XRT);
        });
    }

    #[test]
    fn stale_generation_is_rejected_without_fee_fallback() {
        sp_io::TestExternalities::default().execute_with(|| {
            let who = account(1);
            assert_ok!(Balances::mint_into(&who, 1_000 * XRT));
            let generation = subscribe(&who);
            let balance = Balances::balance(&who);

            let result = ChargeTransaction::<Runtime>::subscription(
                AuthorizationContext::Account(who.clone()),
                generation + 1,
            )
            .validate_and_prepare(
                RawOrigin::Signed(who.clone()).into(),
                &call(),
                &info(),
                LEN,
                0,
            );
            assert_eq!(
                result.err(),
                Some(
                    InvalidTransaction::Custom(pallet_robonomics_transaction::GENERATION_MISMATCH)
                        .into()
                )
            );
            assert_eq!(Balances::balance(&who), balance);
        });
    }

    #[test]
    fn node_funding_needs_a_local_limit_provider() {
        sp_io::TestExternalities::default().execute_with(|| {
            let (owner, actor) = (account(1), account(2));
            assert_ok!(Balances::mint_into(&owner, 1_000 * XRT));
            assert_ok!(CPS::create_node(
                RawOrigin::Signed(owner.clone()).into(),
                None,
                None,
                None,
                None
            ));
            assert_ok!(CPS::grant_access(
                RawOrigin::Signed(owner.clone()).into(),
                NodeId(0),
                actor.clone(),
                Capability::Transaction,
                GrantMode::Node,
            ));
            let generation = subscribe(&owner);

            // Authorized, but the runtime has no local limits yet.
            let result = ChargeTransaction::<Runtime>::subscription(
                AuthorizationContext::Node(NodeId(0)),
                generation,
            )
            .validate_and_prepare(
                RawOrigin::Signed(actor).into(),
                &call(),
                &info(),
                LEN,
                0,
            );
            assert_eq!(result.err(), Some(InvalidTransaction::Payment.into()));
        });
    }
}
