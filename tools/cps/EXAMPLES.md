# 🌳 CPS CLI Examples

This document shows examples of the beautiful CLI output.

## Help Output

```bash
$ cps --help
```

```
🌳 Beautiful CLI for Robonomics CPS (Cyber-Physical Systems)

Usage: cps [OPTIONS] <COMMAND>

Commands:
  show         Display node information and its children in a beautiful tree format
  create       Create a new node (root or child)
  set-meta     Update node metadata
  set-payload  Update node payload
  move         Move a node to a new parent
  remove       Delete a node (must have no children)
  mqtt         MQTT bridge commands
  help         Print this message or the help of the given subcommand(s)

Options:
      --ws-url <WS_URL>          WebSocket URL for blockchain connection
      --suri <SURI>              Account secret URI (e.g., //Alice, //Bob, or seed phrase)
      --mqtt-broker <MQTT_BROKER>  MQTT broker URL
      --mqtt-username <MQTT_USERNAME>  MQTT username
      --mqtt-password <MQTT_PASSWORD>  MQTT password
      --mqtt-client-id <MQTT_CLIENT_ID>  MQTT client ID
  -h, --help                     Print help
  -V, --version                  Print version
```

## Show Command

```bash
$ cps show 0
```

Example output (with live node):

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
🔄 Fetching node 0...

🌳 CPS Node ID: 0

├─ 📝 Owner: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
├─ 📊 Meta: {
     "type": "sensor",
     "location": "room1"
   }
└─ 🔐 Payload: 22.5C

   👶 Children: (3 nodes)
      ├─ NodeId: 1
      ├─ NodeId: 2
      └─ NodeId: 3
```

## Create Command

```bash
$ cps create --meta '{"type":"building"}' --payload '{"status":"online"}'
```

Output:

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
ℹ️  Using account: d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
ℹ️  Creating root node

Example output (with live node):
✅ Node created with ID: 42
```

## Set Payload Command

```bash
$ cps set-payload 5 '23.1C'
```

Output:

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
ℹ️  Updating payload for node 5

Example output (with live node):
✅ Payload updated for node 5
```

## Move Command

```bash
$ cps move 5 3
```

Output:

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
ℹ️  Moving node 5 to parent 3

Example output (with live node):
✅ Node 5 moved to parent 3
```

## Remove Command

```bash
$ cps remove 5
```

Output:

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
⚠️  Are you sure you want to delete node 5? (y/N): y
ℹ️  Deleting node 5

Example output (with live node):
✅ Node 5 deleted
```

## MQTT Subscribe

```bash
$ cps mqtt subscribe "sensors/temp01" 5
```

Output:

```
🔄 Connecting to blockchain...
ℹ️  Connected to ws://localhost:9944
🔄 Connecting to MQTT broker...

Example usage would be:
cps mqtt subscribe sensors/temp01 5 

The bridge would:
• Subscribe to MQTT topic sensors/temp01
• On each message, update node 5 payload
• Store messages as plain text
```

When running with a live MQTT broker:

```
📡 Connecting to MQTT broker...
✅ Connected to mqtt://localhost:1883
📥 Subscribed to topic: sensors/temp01
🔄 Listening for messages...

[2025-12-04 10:30:15] 📨 Received: 22.5C
✅ Updated node 5 payload

[2025-12-04 10:30:45] 📨 Received: 23.1C
✅ Updated node 5 payload
```

## MQTT Publish

```bash
$ cps mqtt publish "actuators/valve01" 10 --interval 5
```

When running with a live MQTT broker and node:

```
📡 Connecting to MQTT broker...
✅ Connected to mqtt://localhost:1883
🔄 Monitoring node 10 payload...

[2025-12-04 10:31:00] 📤 Published to actuators/valve01: {"state":"open"}
[2025-12-04 10:31:05] 📤 Published to actuators/valve01: {"state":"closed"}
```

## Emoji Legend

The CLI uses emojis for visual clarity:

- 🌳 - CPS Tree / Main title
- 🔄 - In progress / Loading
- ✅ - Success
- ❌ - Error
- ⚠️ - Warning
- ℹ️ - Information
- 📝 - Owner information
- 📊 - Metadata
- 🔐 - Payload / Encrypted data
- 👶 - Children nodes
- 📡 - MQTT / Network
- 📥 - Incoming message (subscribe)
- 📤 - Outgoing message (publish)
- 📨 - Message received

## Color Scheme

- **Cyan/Blue**: Informational messages, node IDs, topics
- **Green**: Success messages
- **Red**: Error messages, encrypted data indicators
- **Yellow**: Warnings, examples
- **Magenta**: Metadata
- **White/Bright**: Data content
- **Black/Gray**: Structural elements (tree lines)
