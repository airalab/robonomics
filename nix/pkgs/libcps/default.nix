{
  lib,
  rustPlatform,
  stdenv,
  pkgs,
}:

rustPlatform.buildRustPackage {
  pname = "libcps";
  version = "0.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "tools/libcps";

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
