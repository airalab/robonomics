{
  cacert,
  lib,
  rustc,
  rocksdb,
  openssl,
  pkg-config,
  protobuf,
  rust-jemalloc-sys-unprefixed,
  rustPlatform,
  revHash,
  stdenv,
  pkgs,
}:

rustPlatform.buildRustPackage {
  pname = "robonomics";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "bin";

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    rustc
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
    SKIP_WASM_BUILD = 1;
    OPENSSL_NO_VENDOR = 1;
    PROTOC = "${protobuf}/bin/protoc";
    SUBSTRATE_CLI_GIT_COMMIT_HASH="${revHash}";
    ROCKSDB_LIB_DIR = lib.makeLibraryPath [ rocksdb ];
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
