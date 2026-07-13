{
  cacert,
  lib,
  rocksdb,
  openssl,
  pkg-config,
  protobuf,
  rust-jemalloc-sys-unprefixed,
  rust-toolchain,
  llvmPackages,
  stdenv,
  pkgs,
}:

let
  rustPlatform = pkgs.makeRustPlatform {
    rustc = rust-toolchain;
    cargo = rust-toolchain;
  };
in
rustPlatform.buildRustPackage rec {
  pname = "robonomics";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "bin";

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    llvmPackages.lld
    rust-toolchain
  ];

  # NOTE: jemalloc is used by default on Linux
  buildInputs = [
    openssl
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [ rust-jemalloc-sys-unprefixed ];

  checkInputs = [
    cacert
  ];

  env = {
    OPENSSL_NO_VENDOR = 1;
    PROTOC = "${protobuf}/bin/protoc";
#    ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  };

  meta = with lib; {
    description = "Implementation of a https://robonomics.network node in Rust based on the Substrate framework";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.aarch64 ++ platforms.x86
    );
  };
}
