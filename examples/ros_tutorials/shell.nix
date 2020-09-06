{ nixpkgs ? import ../../nixpkgs.nix { }
}:

with nixpkgs;
with rosPackages.melodic;
with pythonPackages;

mkShell {
  buildInputs = [
    glibcLocales
    (buildEnv { paths = [
      ros-comm
    ]; })
  ];

  ROS_HOSTNAME = "localhost";
  ROS_MASTER_URI = "http://localhost:11311";
}
