{
  perSystem = { self', config, pkgs, lib, ... }: {
    devShells.default = with pkgs; mkShell.override { stdenv = clangStdenv; } {
        inputsFrom = [ self'.devShells.rust ];
        buildInputs = [
          openssl rustfmt taplo actionlint cargo-nextest
          subxt-cli srtool-cli psvm frame-omni-bencher
        ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys ];

        LIBCLANG_PATH = lib.makeLibraryPath [ llvmPackages.libclang ];
        RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${lib.makeBinPath [ protobuf ]}/protoc";
      };

    devShells.local-testnet =
        let robonomics = config.rust-project.crates."robonomics".crane.outputs.drv.crate;
            libcps = config.rust-project.crates."libcps".crane.outputs.drv.crate;
        in with pkgs; mkShell {
          inputsFrom = [ self'.devShells.default ];
          buildInputs = [ polkadot polkadot-parachain zombienet robonomics libcps ];
        };

    devShells.benchmarking = with pkgs; mkShell.override { stdenv = clangStdenv; } {
        inputsFrom = [ self'.devShells.rust ];
        buildInputs = [
          openssl frame-omni-bencher srtool-cli
        ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys ];

        LIBCLANG_PATH = lib.makeLibraryPath [ llvmPackages.libclang ];
        RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${lib.makeBinPath [ protobuf ]}/protoc";
      };
  };
}
