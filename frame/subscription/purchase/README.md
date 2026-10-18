# pallet-robonomics-subscription-purchase

Fixed-product purchase mechanism for
[`pallet-robonomics-subscription`](../README.md). A protocol-configured
product (price, validity, bounded resource limits) is bought by a deposit of
the payment asset and allocated through the generic `SubscriptionAllocator`
interface.

```text
Product { price, valid_for, resources, enabled }
deposit >= price --> SubscriptionAllocator::allocate(depositor, allocation)
deposit <  price --> rejected, the deposit stays with the sender
```

## Extrinsics

| Call | Origin | Description |
| --- | --- | --- |
| `create_product(price, valid_for, resources)` | `ManagementOrigin` | Add an enabled product. |
| `update_product(id, price, valid_for, resources)` | `ManagementOrigin` | Replace a template; affects future purchases only. |
| `disable_product(id)` | `ManagementOrigin` | Stop sales of a product. |

There is no purchase extrinsic: a purchase is the deposit itself, through
`Pallet::purchase_with_deposit(beneficiary, amount)`, which the runtime calls
from its inbound transfer handler. `Config::DepositProduct` selects the
product deposits buy.

## Invariants

- Products are templates. Creating or updating one reserves no capacity;
  capacity admission is performed by the subscription pallet when a purchase
  allocates, and its failure is propagated.
- A purchase is atomic and has no effect on failure: a deposit below the
  price, a missing or disabled product, or a failed allocation all reject
  the deposit.
- The pallet holds no asset and keeps no credit. A deposit that covers the
  price is burned as a whole; an excess is not refunded.
- A purchase replaces the beneficiary's current generation; nothing rolls
  over.
- The subscription pallet never learns the price or the product id, so a
  purchase and a governance allocation of the same allocation produce
  identical subscription state.

## Payment over XCM

The price is expressed in USDT, which stays on AssetHub: the reserve keeps
the real asset in this chain's sovereign account, and the runtime never
creates a local asset (`runtime/robonomics/src/purchase_deposit.rs`). When
AssetHub deposits USDT to a user, the transactor buys the product for the
user's derived account and burns the deposit:

```text
ReserveAssetDeposited(USDT)
BuyExecution(USDT)
DepositAsset(beneficiary = AssetHub/AccountId32(user))
```

If the purchase is rejected the whole `DepositAsset` fails, the USDT stays in
the XCM holding, and the sender recovers it through its own error handler or
appendix; otherwise it is trapped and claimable.

## Weights

`weights::SubstrateWeight` contains conservative estimates. Replace them with
benchmarked weights. The deposit itself is weighed by the runtime's XCM
weigher.
