# Robonomics Migration guide

This short tutorial helps to upgrade Robonomics parachain testnet during version upgrade.

## 0.18.x -> 0.19.0

1. Download precompiled from [release](https://github.com/airalab/robonomics/releases/tag/v0.19.0).

2. Wipe databases.

```
rm -rf $BASE_PATH/chains/robonomics/db
rm -rf $BASE_PATH/polkadot/db
```

where `BASE_PATH=~/.local/share/robonomics/` by default.

3. Launch new binary.
