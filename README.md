Robonomics on Substrate
=======================

> Substrate SRML-based Robonomics node Proof of Concept.

Run
---

```bash
curl https://nixos.org/nix/install | sh
nix-shell --run "./build.sh && cargo run --release -- --chain ./robonomics.json"
```
