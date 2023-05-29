Perequisition.
Generate "./metadata.scale" for application (do it only once)
Having it build application.


1. Run robonomics node:
./target/release/robonomics  --dev

2.  Optional:  run local mqtt broker or connect to some existiong in the next step:
tcp://broker.emqx.io:1883 or tcp://test.mosquitto.org:1883 or tcp://mqtt.eclipseprojects.io:1883 etc.

3. Run digital twin application:
./target/release/digitaltwin2mqtt tcp://broker.emqx.io:1883

Note 1: By default is mqtt broker on tcp://localhost:1883
Note 2: ./metadata.scale should be in folder where application started to run

4. Run script to subscribe to digitaltwin topic at mqtt broker, i.e.: 
python3 mqttc.py broker.emqx.io 1883 

Note: by default is localhost 1883
python3 mqttc.py

5. In web brouser open UI:
https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/extrinsics
Go to for digital twin create() extrinsic. It creates created digital twin with id 0.
Then it will be possible to use setSource() extrinsics for this id.

Note: Next call of create() creates other digital twin id 1 for use, etc.

