{
  lib,
  stdenv,
  protobuf-compiler,
  rustPlatform,
  llvmPackages,
  rocksdb,
  snappy,
  revHash,
  pkgs,
  isStatic ? stdenv.hostPlatform.isStatic,
}:

let
  staticSnappy = pkgs.snappy.overrideAttrs (oldAttrs: {
    cmakeFlags = (oldAttrs.cmakeFlags or []) ++ ["-DBUILD_SHARED_LIBS=OFF"];
  });
  sse42Support = stdenv.hostPlatform.sse4_2Support;
  staticRocksdb = pkgs.rocksdb.overrideAttrs (oldAttrs: {
    cmakeFlags = [
      "-DPORTABLE=1"
      "-DWITH_JEMALLOC=0"
      "-DWITH_LIBURING=0"
      "-DWITH_JNI=0"
      "-DWITH_BENCHMARK_TOOLS=0"
      "-DWITH_TESTS=0"
      "-DWITH_TOOLS=0"
      "-DWITH_CORE_TOOLS=1"
      "-DWITH_BZ2=0"
      "-DWITH_LZ4=0"
      "-DWITH_SNAPPY=1"
      "-DWITH_ZLIB=0"
      "-DWITH_ZSTD=0"
      "-DWITH_GFLAGS=0"
      "-DUSE_RTTI=1"
      "-DROCKSDB_BUILD_SHARED=0"
    ] ++ (lib.optional sse42Support "-DFORCE_SSE42=1") ;
  }) ;
in rustPlatform.buildRustPackage {
  pname = "robonomics";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  nativeBuildInputs = [rustPlatform.bindgenHook]; 

  checkPhase = false;
  buildType = "production";
  buildAndTestSubdir = "bin";

  env = {
    SKIP_WASM_BUILD = 1;
    OPENSSL_NO_VENDOR = 1;
    PROTOC = "${protobuf-compiler}";
    SUBSTRATE_CLI_GIT_COMMIT_HASH = "${builtins.substring 0 7 revHash}";
    ROCKSDB_LIB_DIR = "${if isStatic then staticRocksdb else rocksdb}/lib";
    ROCKSDB_STATIC = if isStatic then "1" else "0";
    SNAPPY_LIB_DIR = "${if isStatic then staticSnappy else snappy}/lib";
    SNAPPY_STATIC = if isStatic then "1" else "0";
    RUSTFLAGS = if isStatic then "-C target-feature=+crt-static -Clink-arg=-lc++ -Clink-arg=-lc++abi" else "";
  };

  meta = with lib; {
    description = "Implementation of a https://robonomics.network node in Rust based on the Substrate framework";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.riscv64 ++ platforms.aarch64 ++ platforms.x86
    );
  };
}
