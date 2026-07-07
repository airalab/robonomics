{
  lib,
  openssl,
  protobuf,
  pkg-config,
  rust-toolchain,
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
  pname = "libcps";
  version = "0.2.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "tools/robonet";

  nativeBuildInputs = [
    rustPlatform.bindgenHook
    rust-toolchain
    pkg-config
  ];

  buildInputs = [
    openssl
  ];

  env = {
    OPENSSL_NO_VENDOR = 1;
    PROTOC = "${protobuf}/bin/protoc";
  };

  meta = with lib; {
    description = "Robonomics local network spawner based on ZombieNet SDK.";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.aarch64 ++ platforms.x86
    );
  };
}
