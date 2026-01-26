{
  description = "Robonomics Network Flakes";

  nixConfig = {
    extra-substituters = [
      "https://polkadot.cachix.org"
      "https://aira.cachix.org"
    ];
    extra-trusted-public-keys = [
      "polkadot.cachix.org-1:qOFthM8M0DTotg8A48wWTZBgJD6h1rV9Jaszt6QE/N0="
      "aira.cachix.org-1:4mMjRo4HgJ8/i/lzXZPjmnndcdf5P2RZJi04359ykrE="
    ];
  };

  inputs = {
    systems.url = "github:nix-systems/default";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";

    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";

    rust-flake.url = "github:juspay/rust-flake";
    rust-flake.inputs.nixpkgs.follows = "nixpkgs";

    polkadot.url = "github:andresilva/polkadot.nix";
    polkadot.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = with builtins;
        map
          (fn: ./nix/modules/flake/${fn})
          (attrNames (readDir ./nix/modules/flake));
  };
}
