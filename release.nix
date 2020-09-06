{ nixpkgs ? import ./nixpkgs.nix { }
}:

with nixpkgs;
with nixpkgs.rosPackages.noetic;

let
  channel = rustChannelOf { date = "2020-07-20"; channel = "nightly"; };
  targets = [ "wasm32-unknown-unknown" ];
in rec {
  rust = channel.rust.override { inherit targets; };
  substrate-ros-msgs = callPackage ./substrate-ros/msgs/substrate_ros_msgs { };

  turtlesim = callPackage ./examples/turtlesim_liability { };
  ros_tutorials = callPackage ./examples/ros_tutorials { };
  node = callPackage ./. { inherit rust substrate-ros-msgs; };
}
