{
  pkgs,
  linker ? "wild",
  packages ? [ ],
  env ? { },
}:

assert pkgs.lib.assertOneOf "linker" linker [
  "mold"
  "wild"
  "lld"
];

let
  cargoLinker =
    with pkgs;
    let
      rustTarget = stdenv.hostPlatform.rust.cargoEnvVarTarget;
      linkerFlags = {
        lld = "-Clink-arg=-fuse-ld=${llvmPackages.lld}/bin/ld.lld -Clink-arg=-Wl,--no-rosegment";
        mold = "-Clink-arg=-fuse-ld=${mold}/bin/ld.mold -Clink-arg=-Wl,--no-rosegment -Clink-arg=-flto";
        wild = "-Clink-arg=-fuse-ld=${wild}/bin/ld.wild -Clink-arg=-flto";
      };
      rustflags =
        if stdenv.isDarwin then
          "-Clink-arg=-fuse-ld=${llvmPackages.lld}/bin/ld64.lld"
        else
          linkerFlags.${linker};
    in
    {
      "CARGO_TARGET_${rustTarget}_LINKER" = "clang";
      "CARGO_TARGET_${rustTarget}_RUSTFLAGS" = rustflags;
    };
in
with pkgs;
mkShell.override { stdenv = clangStdenv; } {
  packages =
    packages
    ++ [
      llvmPackages.lld
      rust-toolchain
    ];
  env = cargoLinker // env;
}
