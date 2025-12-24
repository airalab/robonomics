# Migrating to Robonomics Node v4.x

This guide explains how to upgrade your Robonomics node to **version 4.x**.

## 1. Download binary

Download the official v4.0 binary from GitHub:

* **Release:** `v4.0.4`
* Link: [https://github.com/airalab/robonomics/releases/tag/v4.0.4](https://github.com/airalab/robonomics/releases/tag/v4.0.4)

Replace your existing node binary with the new one.

## 2. Download Parachain Snapshot

Download parachain snapsot from:

* Link: [https://snapshots.robonomics.network/](https://snapshots.robonomics.network/)

Clear your parachain base and extract from archive to ```/path/to/your/parachain/database```.
Fix permissions if necessary.

## 3. Remove the Deprecated `--lighthouse-account` Flag

Starting from v4.0, the `--lighthouse-account` CLI flag is no longer supported.

If your systemd service or startup script contains:

```
--lighthouse-account <ACCOUNT>
```

Remove this line entirely before restarting the node.

## 4. Generate New Session Keys

Robonomics v4.0 follows the updated Polkadot SDK requirements, so collators must generate fresh session keys.

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
   If needed, you can manually insert keys using:

   ```
   author_insertKey
   ```

Restart the node after generating the keys to ensure they are active.

## 5. Register Your Collator On-Chain

Once the node is running with the new session keys, you must provide the session keys (which you generated earlier) and register your collator in the **Collator Selection pallet**.

Typical steps:

1. Prepare your collator account (the controller account):

   Use the account you intend to register as collator (this is the **controller** account).  
   Import it using the **seed phrase** from the keystore file generated in **Step 4** (default location: `<your-base-path>/keystore` or `<your-base-path>/chains/<chain-name>/keystore`).

2. First Submit the extrinsic:

   ```
   session.setKeys(keys, proof)
   ```
   
   - In the **"keys"** field paste the full hex string from `author_rotateKeys` (the one you generated earlier).

   - In the **"proof"** field enter `0x00` (or "space" key).

3. Then Submit the extrinsic:

   ```
   collatorSelection.registerAsCandidate()
   ```

4. Wait for the session change to complete. After that, your node should appear in the candidate list and begin authoring blocks.

