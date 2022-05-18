Pubsub subscribe:
```
import time
import robonomicsinterface as RI
from robonomicsinterface import PubSub

def subscription_handler(obj, update_nr, subscription_id):
    print(obj['params']['result'])
    if update_nr >= 2:
        return 0

interface = RI.RobonomicsInterface(remote_ws="ws://127.0.0.1:9944")
pubsub = PubSub(interface)

print(pubsub.listen("/ip4/127.0.0.1/tcp/44440"))
time.sleep(2)
print(pubsub.connect("/ip4/127.0.0.1/tcp/44441"))
print(pubsub.subscribe("42", result_handler=subscription_handler))
```

Pubsub publish:
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

Nodes:
```
target/debug/robonomics --dev --tmp -l rpc=trace
target/debug/robonomics --dev --tmp --ws-port 9991 -l rpc=trace
```
