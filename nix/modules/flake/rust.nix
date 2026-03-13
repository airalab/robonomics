{ inputs, ... }:

{
  imports = [
    inputs.rust-flake.flakeModules.default
  ];

  perSystem = { self', config, pkgs, lib, ... }: {
    rust-project = {
      src = lib.cleanSourceWith {
        src = inputs.self;
        filter = path: type:
          config.rust-project.crane-lib.filterCargoSources path type
            ## Include chain spec files.
            || (lib.hasInfix "chains/" path && lib.hasSuffix ".json" path)
            ## Include scale metadata.
            || (lib.hasInfix "tools/robonet/artifacts/" path && lib.hasSuffix ".scale" path);
      };
      crates."robonomics".crane.args = with pkgs; {
        nativeBuildInputs = [ pkg-config rustPlatform.bindgenHook ];
        buildInputs = [ openssl ];
#        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys-unprefixed ];
        OPENSSL_STATIC = 1;
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${protobuf}/bin/protoc";
        #ROCKSDB_LIB_DIR = "${rocksdb}/lib";
        #SNAPPY_LIB_DIR = "${snappy}/lib";
        CARGO_BUILD_TARGET = "aarch64-unknown-linux-musl";
        CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
      };
      crates."robonet".crane.args = with pkgs; {
        buildInputs = [ openssl ];
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${protobuf}/bin/protoc";
      };
    };

    packages = let inherit (config.rust-project) crates; in rec {
      default = self'.packages.robonomics;
      polkadot = pkgs.polkadot;
      polkadot-parachain = pkgs.polkadot-parachain;
    };
  };
}
