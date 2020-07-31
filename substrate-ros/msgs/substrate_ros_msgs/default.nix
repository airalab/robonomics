{ stdenv
, catkin 
, buildRosPackage 
, message-generation
, rospy 
, std-msgs
, std-srvs
}:

buildRosPackage rec {
  name = "${pname}-${version}";
  pname = "substrate_ros_msgs";
  version = "master";

  src = ./.;
  buildType = "catkin";
  buildInputs = [ message-generation ];
  propagatedBuildInputs = [ rospy std-msgs std-srvs ];
  nativeBuildInputs = [ catkin ];

  meta = with stdenv.lib; {
    description = "Robonomics Substrate module ROS messages";
    homepage = http://github.com/airalab/substrate-node-robonomics;
    license = licenses.asl20;
    maintainers = [ maintainers.akru ];
  };
}
