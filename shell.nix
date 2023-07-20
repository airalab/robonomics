{ rust-overlay ? import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/master.tar.gz),
  pkgs ? import <nixpkgs> { overlays = [ rust-overlay ]; },
  toolchain ? pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml,
}:

with pkgs;
with llvmPackages_10;

mkShell {
  buildInputs = [
    clang
    toolchain
    pkg-config
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${clang-unwrapped.lib}/lib";
  PROTOC = "${protobuf}/bin/protoc";
}
