# Migrating to Robonomics Node v4.2.0

This guide explains how to upgrade your Robonomics node to **version 4.2.0**.

In this manual we are assuming the following things:
- The service is run on behalf of the `robonomics` user
- The `robonomics` user's home directory is `/var/lib/robonomics/`
- The `base-path` of service is `/var/lib/robonomics/base/` 

## 1. Download Binary

Download the official v4.2.0 binary from GitHub:

* **Release:** `v4.2.0`
* Link: [https://github.com/airalab/robonomics/releases/tag/v4.2.0](https://github.com/airalab/robonomics/releases/tag/v4.2.0)

Download and install:

```bash
wget -o robonomics.tar.gz \
  https://github.com/airalab/robonomics/releases/download/v4.2.0/robonomics-v4.2.0-ubuntu-x86_64.tar.gz
tar -xzf robonomics.tar.gz
sudo mv robonomics /usr/local/bin/
sudo chmod +x /usr/local/bin/robonomics
```

## 2. Download Chain Specification

The v4.2.0 binary does **not** include built-in chain specifications. You must download the chain spec file before starting the node.
We recommend to store it in the robonomics user's home directory (`/var/lib/robonomics/` in this manual)

For **Kusama** parachain:

```bash
wget -o /var/lib/robonomics/robonomics-kusama.raw.json \
  https://raw.githubusercontent.com/airalab/robonomics/refs/heads/master/chains/kusama-parachain.raw.json
```

For **Polkadot** parachain:

```bash
wget -o /var/lib/robonomics/robonomics-polkadot.raw.json \
  https://raw.githubusercontent.com/airalab/robonomics/refs/heads/master/chains/polkadot-parachain.raw.json
```

Use the downloaded file with the `--chain` flag, e.g. `--chain /var/lib/robonomics/robonomics-kusama.raw.json`.

## 3. Generate Network Key

v4.2.0 refuses to start without a valid network key. Generate one before first launch:

```bash
robonomics key generate-node-key \
  --base-path /var/lib/robonomics/base/ \
  --chain /var/lib/robonomics/robonomics-kusama.raw.json
```

Replace the `--chain` value with your chain spec path. 

After this the network key file `/var/lib/robonomics/base/chains/robonomics/network/secret_ed25519` will be appear. Don't forget to save it for the future possible migrations.

## 4. Download Parachain Snapshot (required for Kusama parachain only)

Snapshots are currently available **only for the Kusama parachain**:

* Link: [https://snapshots.robonomics.network/](https://snapshots.robonomics.network/)

Clear your parachain base and extract from archive to `/path/to/your/parachain/database`. In this example this path is `/var/lib/robonomics/base/chains/robonomics/db/`
Fix permissions if necessary.

For the **Polkadot** parachain, use `--sync warp` instead.

## 5. Remove the Deprecated `--lighthouse-account` Flag

Starting from v4.0, the `--lighthouse-account` CLI flag is no longer supported.

If your systemd service or startup script contains:

```
--lighthouse-account <ACCOUNT>
```

Remove this line entirely before restarting the node.

## 6. Recommended Startup Flags

Below is a recommended systemd `ExecStart` configuration:

```
ExecStart=/usr/local/bin/robonomics \
  --name "YOUR_NODE_NAME" \
  --chain /var/lib/robonomics/robonomics-kusama.raw.json \
  --base-path /var/lib/robonomics/base/ \
  --collator \
  --sync warp \
  --trie-cache-size 0 \
  --telemetry-url "wss://telemetry.parachain.robonomics.network/submit/ 0" \
  -- \
  --sync warp
```

Key flags:

* `--sync warp` — enables warp sync for the parachain. Much faster initial sync.
* `--trie-cache-size 0` — disables the trie cache, significantly reducing RAM usage. Recommended by the Robonomics team.
* `--telemetry-url "wss://telemetry.parachain.robonomics.network/submit/ 0"` — the chain spec has `telemetryEndpoints: null`, so telemetry must be enabled explicitly via this flag.
* `-- --sync warp` — enables warp sync for the **relay chain** (after the `--` separator). Without this, the embedded relay chain can take weeks to sync.

## 7. Generate New Session Keys

Robonomics >= v4.0 follows the updated Polkadot SDK requirements, so collators must generate fresh session keys.

**Important:** You must temporarily start the node with `--rpc-methods unsafe` for the `author_rotateKeys` RPC call to work. Remove this flag after generating keys.

To generate session keys:

1. Run the RPC method on your node:

   ```
   curl -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
     http://127.0.0.1:9944
   ```
2. The command returns a hex-encoded public key bundle.
3. Copy this value and store it — you will need it for on-chain registration.
4. Ensure the keys are inserted automatically by the node (this happens when using `author_rotateKeys`).
   You can verify this using the following command:

   ```
   curl --silent --location --request POST 'http://localhost:9944' \
   --header 'Content-Type: application/json' \
   --data-raw '{
    "jsonrpc": "2.0",
    "method": "author_hasSessionKeys",
    "params": ["'"ROTATE_KEYS_RESULT"'"],
    "id": 1
   }' | jq
   ```
   NOTE: Replace `ROTATE_KEYS_RESULT` with the hex-encoded public key you just received from `author_rotateKeys`.

5. **Remove `--rpc-methods unsafe`** from your startup configuration and restart the node.

## 8. Register Your Collator On-Chain

Once the node is running with the new session keys, you must register your collator.

Typical steps:

1. Submit the extrinsic using your pre-generated collator account:

   ```
   session.setKeys(keys, proof)
   ```

   - In the **"keys"** field paste the full hex string from `author_rotateKeys` (the one you generated earlier).

   - In the **"proof"** field enter `0x` (empty bytes).

2. Then Submit the extrinsic using the same collator account as origin:

   ```
   collatorSelection.registerAsCandidate()
   ```
   **Requirement:** The collator account needs **> 32 XRT** free balance to register as a candidate.

3. Wait for the session change to complete. After that, your node should appear in the candidate list and begin authoring blocks.

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

## Per-Block Author Reward

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

### Formula

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
pub const COLLATOR_BLOCK_REWARD: Balance = 4_200_000; // 0.0042 XRT (9 decimals)
```

### When to revisit

The reward should be recalculated and a new runtime upgrade shipped whenever
**any** of the input parameters change significantly:

* the cost of the reference hardware moves materially up or down,
* the minimum desired number of active collators changes,
* the actual average block time drifts (changing the blocks-per-year base),
* the XRT market price moves enough that the resulting USD-equivalent reward
  no longer covers the reference hardware cost plus a 30 % margin.

When updating the reward, change `COLLATOR_BLOCK_REWARD`, bump `spec_version`,
and update both the table above and the unit tests in
`runtime/robonomics/src/lib.rs::author_rewards_tests`.

## Disk Requirements

* **Kusama:** parachain ~235 GB + relay chain ~550 GB (growing). Minimum **1 TB** recommended.
* **Polkadot:** parachain + relay chain ~1.1 TB (growing). Minimum **2 TB** recommended.
* **Running both networks:** minimum **5 TB** recommended.


