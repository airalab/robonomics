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

use robonomics_runtime::{xcm_config::LocationToAccountId, AccountId};
use sp_core::crypto::Ss58Codec;
use xcm::latest::prelude::*;
use xcm_runtime_apis::conversions::LocationToAccountHelper;

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
    use frame_support::view_functions::{ViewFunction, ViewFunctionDispatchError, ViewFunctionId};
    use pallet_robonomics_cps::{
        pallet::{
            HasCapabilityViewFunction, IdentityOfViewFunction, NodeByIdentityViewFunction,
            ResolveScopeViewFunction,
        },
        Capability, Error, IdentityNode, NodeId, NodeIdentity, ResolvedScope,
    };
    use parity_scale_codec::{Decode, Encode};
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
