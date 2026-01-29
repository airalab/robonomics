{ inputs, ... }:

{
  imports = [ inputs.rust-flake.flakeModules.nixpkgs ];
  perSystem = { ... }: {
    nixpkgs.overlays = [ inputs.polkadot.overlays.default ];
  };
}
