{ rustPlatform
, rust-nightly
, substrate-ros-msgs
, llvmPackages
, pkg-config
, protobuf
, rocksdb
, libudev
, clang
, cmake
, lib
}:

rustPlatform.buildRustPackage {
  name = "robonomics";
  src = builtins.path { path = ./.; name = "robonomics-src"; };
  cargoSha256 = null; 
  propagatedBuildInputs = [ substrate-ros-msgs ];
  nativeBuildInputs = [ clang ];
  buildInputs = [
    rust-nightly
    libudev
    cmake
  ];
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";

  meta = with lib; {
    description = "Robonomics Node Implementation";
    homepage = "https://robonomics.network";
    license = licenses.asl20;
    maintainers = with maintainers; [ akru ];
    platforms = platforms.linux;
  };
}
