cmake_minimum_required(VERSION 2.8.3)
project(turtlesim_liability)

find_package(catkin REQUIRED COMPONENTS
  rospy
)

catkin_package()

include_directories(
# include
  ${catkin_INCLUDE_DIRS}
)
