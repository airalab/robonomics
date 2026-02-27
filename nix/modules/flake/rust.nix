{ inputs, ... }:

{
  imports = [
    inputs.rust-flake.flakeModules.default
  ];

  perSystem = { config, pkgs, lib, ... }: {
    rust-project = {
      src = lib.cleanSourceWith {
        src = inputs.self;
        filter = path: type:
          config.rust-project.crane-lib.filterCargoSources path type
            ## Include chain spec files.
            || (lib.hasInfix "chains/" path && lib.hasSuffix ".json" path);
      };
      crates."robonomics".crane.args = with pkgs; {
        nativeBuildInputs = [ rustPlatform.bindgenHook ];
        buildInputs = [ openssl ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys ];
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${protobuf}/bin/protoc";
      };
      crates."robonet".crane.args = with pkgs; {
        buildInputs = [ openssl ];
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${protobuf}/bin/protoc";
      };
    };

    packages = let inherit (config.rust-project) crates; in rec {
      default = crates."robonomics".crane.outputs.drv.crate;
      polkadot = pkgs.polkadot;
      polkadot-parachain = pkgs.polkadot-parachain;
    };
  };
}
