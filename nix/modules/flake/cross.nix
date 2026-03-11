{ inputs, self, ... }:

{
  perSystem = { self', config, system, ... }:
    let
      target = "aarch64-unknown-linux-musl";
      crossPkgs = import inputs.nixpkgs {
        inherit system;
        crossSystem.config = "aarch64-linux";
        overlays = [ (import inputs.rust-overlay) ];
      };
      craneLib = (inputs.crane.mkLib crossPkgs).overrideToolchain (p:
        (p.rust-bin.fromRustupToolchainFile (self + /rust-toolchain.toml)).override {
          targets = [ target ];
        }
      );
      robonomics = craneLib.buildPackage {
        src = config.rust-project.src;
        pname = config.rust-project.src.name;

        strictDeps = true;

        CARGO_BUILD_TARGET = target; 
        CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
      };
    in
    {
      packages = rec {
        robonomics-cross-aarch64-linux = robonomics;
      };
    };
}
