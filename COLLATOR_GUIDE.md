# Robonomics Collator Guidelines 

In this manual we are assuming the following things:
- The service is run on behalf of the `robonomics` user
- The `robonomics` user's home directory is `/var/lib/robonomics/`
- The `base-path` of service is `/var/lib/robonomics/base/` 
- The node is run with the generic [`polkadot-omni-node`](https://crates.io/crates/polkadot-omni-node) binary (the **default and recommended** way to run a Robonomics collator), together with a Robonomics chain spec from the [`chain-spec/`](./chain-spec) directory of this repository.

## Download Binary

The `polkadot-omni-node` is a runtime-agnostic parachain collator shipped as part of the [Polkadot SDK](https://github.com/paritytech/polkadot-sdk) releases. It starts a parachain node purely from a chain spec file (no need for a Robonomics-specific binary).

* **Release:** use the latest `polkadot-stable*` tag from [Polkadot SDK releases](https://github.com/paritytech/polkadot-sdk/releases)

Download and install:

```bash
wget -O polkadot-omni-node \
  https://github.com/paritytech/polkadot-sdk/releases/download/polkadot-stable2506-1/polkadot-omni-node
chmod +x polkadot-omni-node
sudo mv polkadot-omni-node /usr/local/bin/
```

You will also need the Robonomics chain spec for your network, available in [`chain-spec/`](./chain-spec):

* Polkadot: [`chains/polkadot-parachain.raw.json`](./chain-spec/polkadot-parachain.raw.json)
* Kusama (deprecated): [`chains/kusama-parachain.raw.json`](./chain-spec/kusama-parachain.raw.json)

Download the chain spec directly from GitHub, for example for the Polkadot parachain:

```bash
wget -O /var/lib/robonomics/robonomics-polkadot.raw.json \
  https://raw.githubusercontent.com/airalab/robonomics/master/chain-spec/polkadot-parachain.raw.json
```

## Generate Network Key

Collator refuses to start without a valid network key. Generate one before first launch:

```bash
polkadot-omni-node key generate-node-key \
  --base-path /var/lib/robonomics/base/ \
  --chain /var/lib/robonomics/robonomics-polkadot.raw.json
```

Replace the `--chain` value with your chain spec path. 

After this the network key file `/var/lib/robonomics/base/chains/robonomics/network/secret_ed25519` will be appear. Don't forget to save it for the future possible migrations.

## Recommended Startup Flags

Below is a recommended systemd `ExecStart` configuration:

```
ExecStart=/usr/local/bin/polkadot-omni-node \
  --name "YOUR_NODE_NAME" \
  --chain /var/lib/robonomics/robonomics-polkadot.raw.json \
  --base-path /var/lib/robonomics/base/ \
  --collator \
  --sync warp \
  --telemetry-url "wss://telemetry.parachain.robonomics.network/submit/ 0" \
  -- \
  --sync warp
```

Key flags:

* `--sync warp` — enables warp sync for the parachain. Much faster initial sync.
* `--telemetry-url "wss://telemetry.parachain.robonomics.network/submit/ 0"` — the chain spec has `telemetryEndpoints: null`, so telemetry must be enabled explicitly via this flag.
* `-- --sync warp` — enables warp sync for the **relay chain** (after the `--` separator). Without this, the embedded relay chain can take weeks to sync.

## Generate New Session Keys

Robonomics >= v4.0 follows the updated Polkadot SDK requirements, so collators must generate fresh session keys.

> For the official reference on session key generation and management, see the Polkadot documentation:
> [Generate Session Keys](https://docs.polkadot.com/node-infrastructure/run-a-validator/onboarding-and-offboarding/key-management/#generate-session-keys).

**Important:** You must temporarily start the node with `--rpc-methods unsafe` for the `author_rotateKeysWithOwner` RPC call to work. Remove this flag after generating keys.

To generate session keys:

1. Run the RPC method on your node, replacing `INSERT_STASH_ACCOUNT_ID` with your collator's stash (own) account ID:

   ```
   curl -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeysWithOwner","params":["INSERT_STASH_ACCOUNT_ID"]}' \
     http://127.0.0.1:9944
   ```
2. The command returns a JSON object with two fields in the result: `keys` (the hex-encoded session keys) and `proof` (the ownership proof), for example:

   ```json
   {
     "jsonrpc": "2.0",
     "result": {
       "keys": "0xda3861a45e0197f3ca145c2c209f9126e5053fas503e459af4255cf8011d51010",
       "proof": "0x1a2b3c4d5e6f..."
     },
     "id": 1
   }
   ```
3. Save both the `keys` and `proof` values — you will need them for on-chain registration.
4. Ensure the keys are inserted automatically by the node (this happens when using `author_rotateKeysWithOwner`).
   You can verify this using the following command:

   ```
   curl --silent --location --request POST 'http://localhost:9944' \
   --header 'Content-Type: application/json' \
   --data-raw '{
    "jsonrpc": "2.0",
    "method": "author_hasSessionKeys",
    "params": ["'"KEYS_RESULT"'"],
    "id": 1
   }' | jq
   ```
   NOTE: Replace `KEYS_RESULT` with the hex-encoded `keys` value you just received from `author_rotateKeysWithOwner`.

5. **Remove `--rpc-methods unsafe`** from your startup configuration and restart the node.

## Register Your Collator On-Chain

Once the node is running with the new session keys, you must register your collator.

Typical steps:

1. Submit the extrinsic using your pre-generated collator account:

   ```
   session.setKeys(keys, proof)
   ```

   - In the **"keys"** field paste the `keys` value from `author_rotateKeysWithOwner` (the one you generated earlier).

   - In the **"proof"** field paste the `proof` value from `author_rotateKeysWithOwner` (do **not** use `0x` empty bytes — the proof binds the session keys to your stash account).

2. Then Submit the extrinsic using the same collator account as origin:

   ```
   collatorSelection.registerAsCandidate()
   ```
   **Requirement:** The collator account needs **> 32 XRT** free balance to register as a candidate.

3. Wait for the session change to complete. After that, your node should appear in the candidate list and begin authoring blocks.

## Disk Requirements

* **Polkadot:** parachain + relay chain ~1.1 TB (growing). Minimum **2 TB** recommended.
* **Kusama:** parachain ~235 GB + relay chain ~550 GB (growing). Minimum **1 TB** recommended.
* **Running both networks:** minimum **5 TB** recommended.

## Collator Rewards

Collators earn from two sources:

1. **Per-block author reward** — a fixed `0.0042 XRT` is minted directly to the
   block author every block. See [Per-Block Author Reward](#per-block-author-reward)
   below.
2. **Transaction fees and tips** — `100 %` of fees and tips are routed to the
   `PotStake` account managed by `pallet_collator_selection` (see
   `DealWithFees` in `runtime/robonomics/src/lib.rs`); the block author then
   receives a share from that pot when producing a block.

### Hardware Cost Estimate

| | Kusama | Polkadot |
|---|---|---|
| CPU | 8 cores | 8 cores |
| RAM | 32–64 GB | 64 GB |
| Storage | 1 TB NVMe | 2 TB NVMe |
| Estimated cost | ~$80–120/mo | ~$120–150/mo |

### Per-Block Author Reward

Starting from spec_version **43**, the Robonomics Polkadot parachain mints a
**fixed per-block reward of `0.0042 XRT` directly to the block author** (in
addition to any transaction fees and tips collected by `pallet_collator_selection`).

The reward is implemented by an `AuthorRewards` event handler wired into
`pallet_authorship::Config::EventHandler` *before* `CollatorSelection`. It
mints directly into the author's account so the author receives the full
reward, rather than half of it (which would happen if the reward were paid
into the `PotStake` account, since
`pallet_collator_selection::note_author` distributes only **half** of the pot
to the current author).

#### Formula

```
reward_per_block =
    (server_cost_per_year * number_of_collators * 1.3)
    / number_of_blocks_per_year
    / XRT_price
```

Substituting the values from issue #510:

| Parameter              | Value                                 |
| ---------------------- | ------------------------------------- |
| Server cost / year     | `$2_040` (OVH Epyc 4345P, 64 GB, 2×960 GB NVMe) |
| Minimum collators      | `7`                                   |
| Profit margin          | `30 %` (factor `1.3`)                 |
| Avg block time         | `7 s` ⇒ `4_505_143` blocks / year     |
| XRT price              | `$1`                                  |

```
(2_040 * 7 * 1.3) / 4_505_143 / 1  ≈  0.004120624 XRT
                                   ≈  0.0042 XRT  (rounded up)
```

Encoded constant in `runtime/robonomics/src/lib.rs`:

```rust
pub const CollatorBlockReward: Balance = 4_200_000; // 0.0042 XRT (9 decimals)
```
