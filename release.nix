{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;
with nixpkgs.rosPackages.noetic;

let
  channel = rustChannelOf { date = "2020-09-20"; channel = "nightly"; };
in rec {
  rust = channel.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = [ "rustfmt-preview" ];
  };
  substrate-ros-msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };

  turtlesim = callPackage ./examples/turtlesim_liability { };
  ros_tutorials = callPackage ./examples/ros_tutorials { };
  node = callPackage ./. { inherit rust substrate-ros-msgs; };
}
