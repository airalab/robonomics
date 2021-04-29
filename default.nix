{ rustPlatform
, llvmPackages
, pkg-config
, protobuf
, rocksdb
, libudev
, clang
, cmake
, rust
, lib
}:

rustPlatform.buildRustPackage {
  name = "robonomics";
  src = builtins.path { path = ./.; name = "robonomics-src"; };
  cargoSha256 = null; 

  nativeBuildInputs = [ clang ];
  buildInputs = [ rust ];

  # NOTE: We don't build the WASM runtimes since this would require a more
  # complicated rust environment setup. The resulting binary is still useful for
  # live networks since those just use the WASM blob from the network chainspec.
  BUILD_DUMMY_WASM_BINARY = 1;

  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";

  meta = with lib; {
    description = "Robonomics Node";
    homepage = "https://robonomics.network";
    license = licenses.asl20;
    maintainers = with maintainers; [ akru ];
    platforms = platforms.linux;
  };
}
