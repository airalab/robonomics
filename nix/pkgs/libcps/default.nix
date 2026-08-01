{
  lib,
  pkgs,
  stdenv,
  rustPlatform,
  isStatic ? stdenv.hostPlatform.isStatic,
}:

rustPlatform.buildRustPackage {
  pname = "libcps";
  version = "0.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildType = "production";
  buildAndTestSubdir = "tools/libcps";

  env = {
    RUSTFLAGS = if isStatic then "-C target-feature=+crt-static" else "";
  };

  meta = with lib; {
    mainProgram = "cps";
    description = "Interacting with Robonomics CPS (Cyber-Physical Systems) pallet";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.riscv64 ++ platforms.aarch64 ++ platforms.x86
    );
  };
}
