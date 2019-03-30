# -*- coding: utf-8 -*-

import rospy
import rosbag
from std_msgs.msg import String, Duration
from geometry_msgs.msg import Twist, Vector3


bag = rosbag.Bag('./twist.bag', 'w')
msg1 = Twist(Vector3(), Vector3())
msg2 = Twist(Vector3(2, 0, 0), Vector3(0, 0, 2))

try:
    bag.write('/turtle1/cmd_vel', msg1, t=rospy.Time.from_sec(0))
    for i in range(0, 239):
        bag.write('/turtle1/cmd_vel', msg2, t=rospy.Time.from_sec(1 + 0.01*i))
    bag.write('/turtle1/cmd_vel', msg1, t=rospy.Time.from_sec(5))
finally:
    bag.close()
