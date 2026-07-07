# `robonomics-collator-rewards`

A small, runtime-agnostic helper that mints a fixed per-block reward
directly to the block author (collator) every time the runtime's
[`pallet_authorship`] notifies a new author.

It exposes a single generic struct, `AuthorRewards<T, Reward, Currency>`,
that implements `pallet_authorship::EventHandler<T::AccountId,
BlockNumberFor<T>>` and is wired into a runtime's
`pallet_authorship::Config::EventHandler` tuple.

## Why this crate exists

The Robonomics parachain pays its collators a fixed reward per authored
block, derived from the cost of running the reference hardware (see issue
[#510]). The arithmetic and the wiring are independent of any specific
Robonomics runtime, so they live here:

- The runtime crate stays small — it only declares the reward constant
  and binds the generic to its concrete `Runtime` / `Balances`.
- The behaviour is unit-tested against a minimal mock runtime instead of
  the full Robonomics runtime.
- The same logic can be reused by future Robonomics runtimes (e.g. on
  Kusama) without copy-pasting.

## Reward formula (Robonomics on Polkadot)

```text
reward_per_block = (server_cost_per_year × collators × 1.3)
                 / blocks_per_year
                 / XRT_price
                ≈ (2040 × 7 × 1.3) / 4_505_143 / 1
                ≈ 0.0042 XRT
```

Encoded in the smallest unit of XRT (9 decimals): `4_200_000`.

The constant lives in the consuming runtime, not in this crate, because
the right value depends on the chain's hardware/cost assumptions and the
token's decimals. Each consumer picks its own `Reward` constant; this
crate only knows how to mint it.

When any of the inputs (server cost, minimum collator count, XRT price)
changes significantly, update the constant in the runtime and bump
`spec_version`.

## Why mint directly to the author?

`pallet_collator_selection::note_author` only forwards **half** of its
pot to the current author — the other half stays in the pot. Routing
this reward through the pot would silently halve the per-block reward
seen by the collator. Minting directly to the author guarantees the
author receives exactly `Reward::get()` per authored block.

Consequently `AuthorRewards` **must** appear before `CollatorSelection`
in the `pallet_authorship::Config::EventHandler` tuple so the author is
paid regardless of the pot's state.

## Wiring example

```rust,ignore
use frame_support::parameter_types;
use robonomics_collator_rewards::AuthorRewards;

parameter_types! {
    /// 0.0042 XRT per block, 9 decimals — see issue #510 and MIGRATIONS.md.
    pub const CollatorBlockReward: Balance = 4_200_000;
}

pub type CollatorAuthorRewards =
    AuthorRewards<Runtime, CollatorBlockReward, Balances>;

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    // `CollatorAuthorRewards` MUST come first so the author receives the
    // full `CollatorBlockReward` independently of the collator-selection
    // pot.
    type EventHandler = (CollatorAuthorRewards, CollatorSelection);
}
```

## Weight accounting

Each call to `note_author` performs extra storage mutations to mint the reward.

The crate registers that cost as `DispatchClass::Mandatory` weight via
`register_extra_weight_unchecked`, so the parachain's per-block weight limits
remain respected.

**Note:** the hard-coded overhead is calibrated against the Robonomics runtime's
`pallet_balances::WeightInfo::force_set_balance_creating` benchmark; if you use a
different `Currency` implementation, re-audit/benchmark the minting cost.

## Error handling

`mint_into` failure is logged via `defensive!` rather than panicking.
With a sensible reward (well above the existential deposit) and
`u128` total issuance the failure path is unreachable in practice;
treating it defensively avoids bricking block production on an
unexpected runtime invariant violation.

## Testing

```sh
cargo test -p robonomics-collator-rewards
```

The tests run against a minimal mock runtime that contains only
`frame_system` and `pallet_balances`.

[`pallet_authorship`]: https://docs.rs/pallet-authorship
[#510]: https://github.com/airalab/robonomics/issues/510
