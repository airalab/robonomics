{ moz_overlay ? builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz
, ros_overlay ? builtins.fetchTarball https://github.com/lopsided98/nix-ros-overlay/archive/master.tar.gz
}:

let
  pkgs = import <nixpkgs> {
    overlays = [
      (import moz_overlay)
      (import "${ros_overlay}/overlay.nix")
    ];
  };
  rust-nightly = pkgs.rustChannelOfTargets "nightly" "2021-11-02" [ "wasm32-unknown-unknown" ];
in
  with pkgs;
  with rosPackages.noetic;
rec {
  ros_tutorials = callPackage ./examples/ros_tutorials { };
  turtlesim = callPackage ./examples/turtlesim_liability { };

  robonomics = callPackage ./. { rust = rust-nightly; };

  inherit pkgs rust-nightly;
}
