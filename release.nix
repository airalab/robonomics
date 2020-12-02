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
  rust-channel = pkgs.rustChannelOf { date = "2020-09-20"; channel = "nightly"; };
in
  with pkgs;
  with rosPackages.noetic;
rec {
  substrate-ros-msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };
  ros_tutorials = callPackage ./examples/ros_tutorials { };
  turtlesim = callPackage ./examples/turtlesim_liability { };

  rust-nightly = rust-channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "rustfmt-preview" ];
  };

  robonomics = callPackage ./. { inherit rust-nightly substrate-ros-msgs; };

  inherit pkgs;
}
