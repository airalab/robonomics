# -*- coding: utf-8 -*-

import rospy
import rosbag
from std_msgs.msg import String


bag = rosbag.Bag('./objective_string.bag', 'w')
msg = String(data='Dear turtle, please make a circle. Thanks!') # let's being polite with a turtle

try:
    bag.write('/turtle1/cmd', msg, t=rospy.Time.from_sec(0.1))
finally:
    bag.close()
