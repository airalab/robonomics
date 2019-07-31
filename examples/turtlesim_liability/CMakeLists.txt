cmake_minimum_required(VERSION 2.8.3)
project(turtlesim_liability)

find_package(catkin REQUIRED COMPONENTS
  rospy
)

catkin_package()

install(
    FILES
        launch/trader.launch
        launch/worker.launch
    DESTINATION
        ${CATKIN_PACKAGE_SHARE_DESTINATION}/launch
)

catkin_install_python(
    PROGRAMS
        scripts/trader_node
        scripts/worker_node
    DESTINATION
        ${CATKIN_PACKAGE_BIN_DESTINATION}
)

include_directories(
# include
  ${catkin_INCLUDE_DIRS}
)
