{
  cacert,
  lib,
  rocksdb,
  openssl,
  pkg-config,
  protobuf,
  rust-jemalloc-sys-unprefixed,
  llvmPackages,
  rustPlatform,
  rustc,
  stdenv,
  pkgs,
}:

rustPlatform.buildRustPackage {
  pname = "robonomics";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  preBuild = ''
    export SUBSTRATE_CLI_GIT_COMMIT_HASH=$(cd ../../.. && git rev-parse --short HEAD)
  '';

  buildType = "production";
  buildAndTestSubdir = "bin";

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    rustc
    llvmPackages.lld
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
    ROCKSDB_LIB_DIR = "${rocksdb}/lib";
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
