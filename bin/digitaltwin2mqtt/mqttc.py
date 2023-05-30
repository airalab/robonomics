import paho.mqtt.client as mqtt
import sys

# The callback for when the client receives a CONNACK response from the server.
def on_connect(client, userdata, flags, rc):
    print("Connected with result code "+str(rc))

    # Subscribing in on_connect() means that if we lose the connection and
    # reconnect then subscriptions will be renewed.
    # client.subscribe("$SYS/#")
    client.subscribe('digitaltwin')

# The callback for when a PUBLISH message is received from the server.
def on_message(client, userdata, msg):
    print("at topic" + msg.topic + " data: " + str(msg.payload))

client = mqtt.Client()
client.on_connect = on_connect
client.on_message = on_message

mqtt_srv =  "localhost"
mqtt_port = 1883

if len(sys.argv) > 1 :
    mqtt_srv = sys.argv[1]

if len(sys.argv) > 2 :
    mqtt_srv =  sys.argv[1]
    mqtt_port = sys.argv[2]

client.connect(mqtt_srv, mqtt_port,60)

# Blocking call that processes network traffic, dispatches callbacks and
# handles reconnecting.
# Other loop*() functions are available that give a threaded interface and a
# manual interface.
client.loop_forever()
