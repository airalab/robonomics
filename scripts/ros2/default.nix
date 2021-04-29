{ release ? import ../../release.nix { }
}:

with release.pkgs;
with rosPackages.foxy;

stdenv.mkDerivation { 
  name = "ros2-nix-shell";
  propagatedBuildInputs = [ action-tutorials-cpp action-tutorials-interfaces action-tutorials-py ament-cmake angles composition demo-nodes-cpp demo-nodes-cpp-native demo-nodes-py depthimage-to-laserscan dummy-map-server dummy-robot-bringup dummy-sensors examples-rclcpp-minimal-action-client examples-rclcpp-minimal-action-server examples-rclcpp-minimal-client examples-rclcpp-minimal-composition examples-rclcpp-minimal-publisher examples-rclcpp-minimal-service examples-rclcpp-minimal-subscriber examples-rclcpp-minimal-timer examples-rclcpp-multithreaded-executor examples-rclpy-executors examples-rclpy-minimal-action-client examples-rclpy-minimal-action-server examples-rclpy-minimal-client examples-rclpy-minimal-publisher examples-rclpy-minimal-service examples-rclpy-minimal-subscriber image-tools intra-process-demo joy lifecycle logging-demo pcl-conversions pendulum-control pendulum-msgs quality-of-service-demo-cpp quality-of-service-demo-py ros-base rqt-common-plugins rviz-default-plugins rviz2 teleop-twist-joy teleop-twist-keyboard tlsf tlsf-cpp topic-monitor turtlesim qt5.full colcon ];
  RMW_IMPLEMENTATION = "rmw_cyclonedds_cpp";
}
