# Pallet Robonomics Claim

Ethereum-to-Substrate token claim system using ECDSA signature verification.

## Overview

The Robonomics Claim pallet enables users who hold Ethereum addresses to claim tokens on the Robonomics parachain by proving ownership of their Ethereum address through cryptographic signatures. This is particularly useful for:

- Token migrations from Ethereum to Substrate-based chains
- Airdrops to Ethereum address holders
- Cross-chain token distributions
- Retroactive rewards for Ethereum users

### Key Features

- **Unsigned Claims**: Users submit unsigned transactions with Ethereum ECDSA signatures
- **Signature Verification**: Validates signatures using Ethereum's `personal_sign` format
- **One-time Claims**: Each Ethereum address can only claim tokens once
- **Governance Control**: New claims can be added via root/governance origin
- **Genesis Support**: Initial claims can be configured at chain genesis
- **Event Emission**: All successful claims emit events for easy tracking

## How It Works

The pallet maintains a mapping of Ethereum addresses to claimable token amounts. When a user wants to claim:

1. User signs their destination account ID with their Ethereum private key
2. User submits the signature along with the destination account via the `claim` extrinsic
3. The pallet verifies the signature matches the claimed Ethereum address
4. Tokens are transferred from the pallet account to the destination account
5. The claim is removed from storage (preventing double claims)
6. A `Claimed` event is emitted

## Configuration

### Adding to Runtime

Add the pallet to your runtime's `Cargo.toml`:

```toml
[dependencies]
pallet-robonomics-claim = { path = "../frame/claim", default-features = false }

[features]
default = ["std"]
std = [
    # ... other pallets
    "pallet-robonomics-claim/std",
]
```

### Runtime Configuration

Configure the pallet in your runtime:

```rust
use frame_support::{parameter_types, PalletId};

parameter_types! {
    // Prefix prepended to signed messages for context separation
    pub const ClaimPrefix: &'static [u8] = b"Pay RWS to the Robonomics account:";
    
    // Pallet ID for deriving the account that holds claimable tokens
    pub const ClaimPalletId: PalletId = PalletId(*b"py/claim");
}

impl pallet_robonomics_claim::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Prefix = ClaimPrefix;
    type PalletId = ClaimPalletId;
    type WeightInfo = pallet_robonomics_claim::TestWeightInfo; // Use benchmarked weights in production
}

// Add to construct_runtime! macro
construct_runtime!(
    pub enum Runtime {
        // ... other pallets
        Claims: pallet_robonomics_claim,
    }
);
```

### Genesis Configuration

Set up initial claims at genesis:

```rust
use pallet_robonomics_claim::{GenesisConfig, EthereumAddress};
use hex_literal::hex;

GenesisConfig {
    claims: vec![
        // Ethereum address => Claimable amount
        (EthereumAddress(hex!["1234567890123456789012345678901234567890"]), 1000 * UNIT),
        (EthereumAddress(hex!["abcdefabcdefabcdefabcdefabcdefabcdefabcd"]), 5000 * UNIT),
    ],
}
```

## Usage

### For Users: Claiming Tokens

#### Step 1: Check Your Claim

First, verify you have a claim:

```javascript
// Using Polkadot.js API
const ethereumAddress = '0x1234567890123456789012345678901234567890';
const claim = await api.query.claims.claims(ethereumAddress);

if (claim.isSome) {
  console.log(`You can claim ${claim.unwrap()} tokens`);
}
```

#### Step 2: Sign Your Account ID

Sign your Substrate account ID with your Ethereum private key:

**Using web3.js:**
```javascript
const Web3 = require('web3');
const { u8aToHex } = require('@polkadot/util');

const web3 = new Web3();
const substrateAccountId = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';

// Convert account ID to bytes (removing '0x' prefix if present)
const accountBytes = api.createType('AccountId', substrateAccountId).toU8a();

// Sign using Ethereum's personal_sign
const signature = await web3.eth.personal.sign(
  u8aToHex(accountBytes),
  ethereumAddress,
  ethereumPassword // or use MetaMask
);

console.log('Signature:', signature);
```

**Using ethers.js:**
```javascript
const { ethers } = require('ethers');
const { u8aToHex } = require('@polkadot/util');

const wallet = new ethers.Wallet(privateKey);
const accountBytes = api.createType('AccountId', substrateAccountId).toU8a();

// The prefix "Pay RWS to the Robonomics account:" is automatically added
const message = u8aToHex(accountBytes);
const signature = await wallet.signMessage(ethers.utils.arrayify(message));

console.log('Signature:', signature);
```

**Using MetaMask:**
```javascript
const { u8aToHex } = require('@polkadot/util');

// Request account access
const accounts = await ethereum.request({ method: 'eth_requestAccounts' });
const ethereumAddress = accounts[0];

const accountBytes = api.createType('AccountId', substrateAccountId).toU8a();
const message = u8aToHex(accountBytes);

// Sign with MetaMask
const signature = await ethereum.request({
  method: 'personal_sign',
  params: [message, ethereumAddress],
});

console.log('Signature:', signature);
```

#### Step 3: Submit the Claim

Submit an unsigned transaction with your signature:

```javascript
// Using Polkadot.js API
const destinationAccount = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const ecdsaSignature = signature; // From Step 2

// Submit unsigned transaction
await api.tx.claims
  .claim(destinationAccount, ecdsaSignature)
  .send();
```

#### Complete Example with Polkadot.js

```javascript
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { ethers } = require('ethers');
const { u8aToHex } = require('@polkadot/util');

async function claimTokens(ethereumPrivateKey, destinationAccount) {
  // Connect to Robonomics node
  const api = await ApiPromise.create({
    provider: new WsProvider('wss://kusama.rpc.robonomics.network')
  });

  // Create Ethereum wallet
  const wallet = new ethers.Wallet(ethereumPrivateKey);
  const ethereumAddress = wallet.address;
  
  console.log('Ethereum address:', ethereumAddress);
  console.log('Destination account:', destinationAccount);

  // Check if claim exists
  const claim = await api.query.claims.claims(ethereumAddress);
  if (!claim.isSome) {
    throw new Error('No claim found for this Ethereum address');
  }
  console.log('Claimable amount:', claim.unwrap().toString());

  // Create signature
  const accountBytes = api.createType('AccountId', destinationAccount).toU8a();
  const message = u8aToHex(accountBytes);
  const signature = await wallet.signMessage(ethers.utils.arrayify(message));
  
  console.log('Signature created:', signature);

  // Submit claim
  const tx = api.tx.claims.claim(destinationAccount, signature);
  
  // Wait for confirmation
  await new Promise((resolve, reject) => {
    tx.send((result) => {
      console.log('Transaction status:', result.status.type);
      
      if (result.status.isInBlock) {
        console.log('Included in block:', result.status.asInBlock.toHex());
      }
      
      if (result.status.isFinalized) {
        console.log('Finalized in block:', result.status.asFinalized.toHex());
        
        // Check for success
        const success = result.events.some(({ event }) =>
          api.events.claims.Claimed.is(event)
        );
        
        if (success) {
          console.log('✅ Claim successful!');
          resolve();
        } else {
          reject(new Error('Claim failed'));
        }
      }
    }).catch(reject);
  });

  await api.disconnect();
}

// Usage
claimTokens(
  '0x1234...', // Ethereum private key
  '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY' // Substrate destination account
);
```

### For Governance: Managing Claims

#### Adding a New Claim

Add a claim for an Ethereum address (requires root origin):

```rust
// Using sudo
Claims::add_claim(
    RuntimeOrigin::root(),
    EthereumAddress(hex!["1234567890123456789012345678901234567890"]),
    1000 * UNIT, // Amount to claim
)?;
```

Via governance proposal:

```javascript
// Using Polkadot.js API
const proposal = api.tx.claims.addClaim(
  ethereumAddress,
  amount
);

// Submit as democracy proposal or council motion
const proposalHash = await api.tx.democracy.propose(
  proposal,
  proposalValue
).signAndSend(proposer);
```

#### Funding the Pallet Account

The pallet account must hold sufficient tokens for all claims:

```javascript
// Calculate pallet account
const palletId = 'py/claim'; // Must match runtime config
const palletAccount = u8aToHex(
  new Uint8Array([...Buffer.from('modl'), ...Buffer.from(palletId)])
).padEnd(66, '0');

// Transfer tokens to pallet account
await api.tx.balances
  .transfer(palletAccount, totalClaimAmount)
  .signAndSend(funder);
```

## Architecture

### Storage

- **Claims**: Map of Ethereum addresses to claimable amounts
  ```rust
  StorageMap<_, Identity, EthereumAddress, Balance>
  ```

### Extrinsics

1. **claim(dest, ethereum_signature)** - Unsigned
   - Allows anyone to submit a claim with an Ethereum signature
   - Validates signature and transfers tokens
   - Removes claim after successful processing

2. **add_claim(who, value)** - Root only
   - Adds a new claim or updates an existing one
   - Used by governance to manage claims

### Events

- **Claimed { who, ethereum_address, amount }**
  - Emitted when tokens are successfully claimed
  - Contains destination account, Ethereum address, and claimed amount

### Errors

- **InvalidEthereumSignature**
  - Signature verification failed
  - Message format incorrect

- **SignerHasNoClaim**
  - No claim exists for the recovered Ethereum address
  - Claim may have already been processed

## Security Considerations

### Signature Verification

The pallet uses Ethereum's standard message signing format:
```
\x19Ethereum Signed Message:\n{length}{prefix}{message}
```

This prevents:
- Cross-chain replay attacks (via prefix)
- Phishing attacks (users see the prefix in their wallet)
- Message confusion (length prefix prevents extension attacks)

### One-time Claims

Each Ethereum address can only claim once. The claim is removed from storage after successful processing, preventing:
- Double-spending
- Replay attacks
- Duplicate claims

### Pallet Account Security

- The pallet account holds all claimable tokens
- Only valid signatures can trigger transfers
- Claims can only be added by root/governance
- Account balance should equal sum of all outstanding claims

### Best Practices

1. **Prefix Configuration**: Use a descriptive prefix that users will see in their wallet
2. **Amount Verification**: Ensure pallet account has sufficient funds before adding claims
3. **Genesis Setup**: Pre-fund pallet account at genesis if using genesis claims
4. **Monitoring**: Track `Claimed` events to monitor claim activity
5. **Testing**: Always test the complete claim flow on testnet first

## Testing

Run the pallet tests:

```bash
cargo test -p pallet-robonomics-claim
```

Run benchmarks:

```bash
cargo build --release --features runtime-benchmarks
./target/release/robonomics benchmark pallet \
  --chain=dev \
  --pallet=pallet_robonomics_claim \
  --extrinsic='*' \
  --steps=50 \
  --repeat=20
```

## Troubleshooting

### Common Issues

**Claim not found**
- Verify the Ethereum address is correct
- Check if claim was already processed
- Confirm claims were properly initialized at genesis or added via `add_claim`

**Invalid signature**
- Ensure you're signing the correct account ID (as bytes, not string)
- Verify the prefix matches the runtime configuration
- Check that you're using `personal_sign` (not `eth_sign` directly)
- Make sure the account ID encoding is correct

**Insufficient balance**
- Pallet account must be funded before claims can be processed
- Check pallet account balance: `api.query.system.account(palletAccount)`

**Transaction fails**
- Verify signature format (65 bytes: r + s + v)
- Check that the destination account is valid
- Ensure claim amount doesn't exceed pallet account balance

## Examples

See the `tests.rs` file for comprehensive examples of:
- Setting up claims at genesis
- Signing messages with Ethereum keys
- Processing valid and invalid claims
- Testing error conditions

## License

Licensed under the Apache License, Version 2.0. See LICENSE file for details.

## References

- [Polkadot Claim Pallet](https://github.com/paritytech/polkadot/tree/master/runtime/common/src/claims.rs) - Original inspiration
- [Ethereum Signed Messages](https://eips.ethereum.org/EIPS/eip-191) - EIP-191 standard
- [Substrate Documentation](https://docs.substrate.io/) - General Substrate/FRAME development
