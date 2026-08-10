{
  lib,
  stdenv,
  rustPlatform,
  rust-jemalloc-sys-unprefixed,
  llvmPackages,
  protobuf,
  rocksdb,
  revHash,
  pkgs,
}:

rustPlatform.buildRustPackage {
  pname = "robonomics";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  nativeBuildInputs = [rustPlatform.bindgenHook];
  buildInputs = [rocksdb] ++ lib.optionals stdenv.hostPlatform.isLinux [
    rust-jemalloc-sys-unprefixed
  ];

  buildType = "production";
  buildAndTestSubdir = "bin";

  env = {
    SKIP_WASM_BUILD = 1;
    OPENSSL_NO_VENDOR = 1;
    PROTOC = "${protobuf}/bin/protoc";
    SUBSTRATE_CLI_GIT_COMMIT_HASH = "${builtins.substring 0 7 revHash}";
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
