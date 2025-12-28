{
  description = "Robonomics Network Node";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [(import inputs.rust-overlay)];
        pkgs = import inputs.nixpkgs {
          inherit system overlays;
        };
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in with pkgs; {
        packages.default = rustPlatform.buildRustPackage rec {
          pname = "robonomics";
          version = "4.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = [toolchain pkg-config clang openssl taplo];
          ROCKSDB_LIB_DIR = "${rocksdb}/lib";
          LIBCLANG_PATH = "${libclang.lib}/lib";
          PROTOC = "${protobuf}/bin/protoc";
        }; 
        devShells.default = mkShell {
          buildInputs = [toolchain pkg-config clang openssl taplo];
          ROCKSDB_LIB_DIR = "${rocksdb}/lib";
          LIBCLANG_PATH = "${libclang.lib}/lib";
          PROTOC = "${protobuf}/bin/protoc";
        };
      }
    );
}

