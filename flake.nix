{
  description = "Robonomics Network Flakes";

  nixConfig = {
    extra-substituters = [
      "https://fenix.cachix.org"
      "https://polkadot.cachix.org"
      "https://robonomics.cachix.org"
    ];
    extra-trusted-public-keys = [
      "fenix.cachix.org-1:ecJhr+RdYEdcVgUkjruiYhjbBloIEGov7bos90cZi0Q="
      "polkadot.cachix.org-1:qOFthM8M0DTotg8A48wWTZBgJD6h1rV9Jaszt6QE/N0="
      "robonomics.cachix.org-1:H3FwZ3khWXfEZ2OlPEiqRenpW1pDMAgRRRXMoksO2Bw="
    ];
  };

  inputs = {
    systems.url = "github:nix-systems/default";
    nixpkgs.url = "github:NixOS/nixpkgs/05988b07fb05cbcb50be6bce197b4b5f75b5e61b";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";

    polkadot.url = "github:andresilva/polkadot.nix";
    polkadot.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
      fenix,
      polkadot,
      ...
    }:
    let 
      mkPkgs = system: nixpkgs.legacyPackages.${system}.appendOverlays [
        polkadot.overlays.default
        (final: prev: {
          rust-toolchain = fenix.packages.${system}.fromToolchainFile { 
            file = ./rust-toolchain.toml;
            sha256 = "sha256-SBKjxhC6zHTu0SyJwxLlQHItzMzYZ71VCWQC2hOzpRY=";
          };
        })
      ];
      eachSystem = f: nixpkgs.lib.genAttrs (import systems) (system: f system (mkPkgs system));
    in {
      checks = eachSystem (
        system: pkgs: {
          buildAll = pkgs.symlinkJoin {
            name = "build-all-packages";
            paths = builtins.attrValues self.packages.${system};
          };
        }
      );

      lib = eachSystem (system: pkgs: {
        mkDevShell = args: import ./shell.nix ({ inherit pkgs; } // args);
      });

      devShells = eachSystem (
        system: pkgs: rec {
          default = self.lib.${system}.mkDevShell {
            packages = with pkgs; [
              openssl taplo actionlint cargo-nextest cargo-audit
              psvm try-runtime-cli subxt-cli srtool-cli frame-omni-bencher
              pkgs.polkadot polkadot-parachain
            ];
            env.RUSTC_WRAPPER = pkgs.lib.getExe pkgs.sccache;
          }; 
          benchmarking = self.lib.${system}.mkDevShell {
            packages = with pkgs; [ frame-omni-bencher ];
          }; 
          robonet = with pkgs; mkShell {
            buildInputs = [ robonomics libcps ];
          };
        }
      );

      packages = eachSystem (system: pkgs: import ./nix/pkgs { inherit pkgs self; });
    }
    // {
      overlays = {
        default = final: prev: import ./overlay.nix final prev;
      };
    };
}
