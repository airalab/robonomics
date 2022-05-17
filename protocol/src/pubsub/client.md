#Pubsub subscribe:

```
import time
import robonomicsinterface as RI
from robonomicsinterface import PubSub

# def subscription_handler(obj, update_nr, subscription_id):
#     print(obj['params']['result'])
#     if update_nr > 2:
#         print("unsubscribe:", pubsub.unsubscribe(subscription_id))

interface = RI.RobonomicsInterface(remote_ws="ws://127.0.0.1:9944")
pubsub = PubSub(interface)

# print(pubsub.listen("/ip4/127.0.0.1/tcp/44440"))
# time.sleep(2)
# print(pubsub.connect("/ip4/127.0.0.1/tcp/44441"))

# subscribe = pubsub.subscribe("42", result_handler=subscription_handler)
subscribe = pubsub.subscribe("42")
print("subscribe:", subscribe)
sid = subscribe.get("result")
print("sid:", sid)
time.sleep(4)
print("unsubscribe:", pubsub.unsubscribe(sid))
```

#Pubsub publish:

```
import time
import robonomicsinterface as RI
from robonomicsinterface import PubSub

interface = RI.RobonomicsInterface(remote_ws="ws://127.0.0.1:9991")
pubsub = PubSub(interface)

print(pubsub.listen("/ip4/127.0.0.1/tcp/44441"))
time.sleep(2)
print(pubsub.connect("/ip4/127.0.0.1/tcp/44440"))

for i in range(10):
    time.sleep(2)
    print("publish:", pubsub.publish("42", "message_" + str(time.time())))
```
