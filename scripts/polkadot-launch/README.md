Playing with Polkadot Launch 
============================

Robonomics parachains could be locally instantiated using **Polkadot Launch** tool.

Install
-------

The first, clone and build polkadot-launch sources:

```bash
git clone https://github.com/shawntabrizi/polkadot-launch
cd polkadot-launch && yarn && yarn build
```

Next step is build and copy robonomics binary into `bin` directory.

```bash
cargo build --release
cp target/release/robonomics scripts/polkadot-launch/bin
```

Final step is build polkadot with `real-overseer` feature enabled and copy binary into `bin`.

```bash
cd $POLKADOT
cargo build --release --features real-overseer
cp target/release/polkadot $ROBONOMICS/scripts/polkadot-launch
```

Launch
------

Scripts automatically instantiate four polkadot validators and two independent robonomics parachains.

```bash
cd scripts/polkadot-launch
node dist/index.js config.json
```

When launch successful the log files with names `alice.log`, `bob.log` ... will be created for polkadot validators.
For parachain collators will be created log with parachain id as the name, for example `200.log`.
