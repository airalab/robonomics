{
  lib,
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
  version = "0.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "tools/libcps";

  nativeBuildInputs = [
    rustPlatform.bindgenHook
    rust-toolchain
  ];

  meta = with lib; {
    description = "Interacting with Robonomics CPS (Cyber-Physical Systems) pallet";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.aarch64 ++ platforms.x86
    );
  };
}
