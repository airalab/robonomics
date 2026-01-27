{
  perSystem = { self', config, pkgs, lib, ... }: {
    devShells.default = with pkgs; mkShell.override { stdenv = clangStdenv; } {
        inputsFrom = [ self'.devShells.rust ];
        buildInputs = [
          openssl rustfmt taplo actionlint
          subxt-cli srtool-cli psvm frame-omni-bencher
        ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys ];

        LIBCLANG_PATH = lib.makeLibraryPath [ llvmPackages.libclang ];
        RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${lib.makeBinPath [ protobuf ]}/protoc";
      };

    devShells.local-testnet = with pkgs; mkShell {
      buildInputs =
        let robonomics = config.rust-project.crates."robonomics".crane.outputs.drv.crate;
        in [ polkadot polkadot-parachain zombienet robonomics ];
    };
  };
}
