{ stdenv
, ros_comm
, mkRosPackage
}:

mkRosPackage rec {
  name = "${pname}-${version}";
  pname = "robonomics_msgs";
  version = "master";

  src = ./.;

  propagatedBuildInputs = [ ros_comm ];

  meta = with stdenv.lib; {
    description = "Robonomics on Substrate messages";
    homepage = http://github.com/airalab/substrate-node-robonomics;
    license = licenses.asl20;
    maintainers = [ maintainers.akru ];
  };
}
