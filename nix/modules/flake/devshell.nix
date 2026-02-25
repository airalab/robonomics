{
  perSystem = { self', config, pkgs, pkgs', lib, ... }:
  let defaultShell = with pkgs; mkShell.override { stdenv = clangStdenv; } {
        inputsFrom = [ self'.devShells.rust ];
        buildInputs = [
          openssl rustfmt taplo actionlint cargo-nextest psvm
          try-runtime-cli subxt-cli srtool-cli frame-omni-bencher
          polkadot polkadot-parachain
        ]
        ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys ];

        LIBCLANG_PATH = lib.makeLibraryPath [ llvmPackages.libclang ];
        RUST_SRC_PATH = "${rustPlatform.rustLibSrc}";
        OPENSSL_NO_VENDOR = 1;
        PROTOC = "${lib.makeBinPath [ protobuf ]}/protoc";
      };
  in {
    devShells.default = defaultShell;

    devShells.localnet =
      let robonomics = config.rust-project.crates."robonomics".crane.outputs.drv.crate;
          libcps = config.rust-project.crates."libcps".crane.outputs.drv.crate;
          localnet = config.rust-project.crates."localnet".crane.outputs.drv.crate;
      in pkgs.mkShell {
        inputsFrom = [ defaultShell ];
        buildInputs = [ robonomics libcps localnet ];
      };

    devShells.benchmarking = with pkgs; mkShell {
        inputsFrom = [ self'.devShells.rust ];
        buildInputs = [ frame-omni-bencher ];
      };
  };
}
