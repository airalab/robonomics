{
  lib,
  makeRustPlatform,
  rust-toolchain,
  llvmPackages,
  stdenv,
  clang,
}:

let rustPlatform = makeRustPlatform {
  rustc = rust-toolchain;
  cargo = rust-toolchain;
};
in rustPlatform.buildRustPackage {
  pname = "robonomics-metadata";
  version = "4.3.0";

  cargoLock.lockFile = ../../../Cargo.lock;
  src = lib.cleanSource ../../..;

  buildAndTestSubdir = "runtime/robonomics/subxt-api";
  buildFeatures = ["build-metadata"];

  postInstall = ''
    mkdir -p $out
    install -Dm644 target/$cargoBuildType/metadata.scale $out/metadata.scale
  '';

  meta = with lib; {
    description = "Robonomics runtime metadata";
    license = licenses.asl20;
    homepage = "https://github.com/airalab/robonomics";
    maintainers = with maintainers; [ akru ];
    platforms = intersectLists platforms.unix (
      platforms.aarch64 ++ platforms.x86
    );
  };
}
