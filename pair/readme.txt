1. Generate 2 private.pk8 keys (see link.txt how to do this): 
first one for server 
second for client

From keys will be extracted peerId and they are static (not random from run to run).

2. Run two examples of robonomics (or one reqres example) from different folders  (or in different PCs, not tested, see note)
where place different keys:

- one instance of robonomics as reqres server from one drectory with server private.pk8 key:
$ ./robonomics pair listen --peer Qma7vLWTmTnwcXfLF4iKEuJqvfCZGRrseYTuxk2GdVE9fZ

- other one instance of robonomics as reqres client from other directory with client private.pk8 key:
$ ../robonomics pair connect --peer QmUnozNz2tmzDeckoShMHbvGp1yWi4BKBiudkELwoKCdcL

In case if reqres server mathes address from command line and peer trying to connect it continue communication.
In other case it reject connection (with message about saving unexpected peer Id).


Notes:
Adrees is hardcoded to connect with /ip4/127.0.0.1/tcp/61241, while no discovery at present time

Robomomics and reqres example to pair:

a) connect to known peer ID, i.e. reqresp exaple node QmUnozNz2tmzDeckoShMHbvGp1yWi4BKBiudkELwoKCdcL
target/release/robonomics-request-response-example /ip4/127.0.0.1/tcp/61241
target/release/robonomics pair connect --peer QmUnozNz2tmzDeckoShMHbvGp1yWi4BKBiudkELwoKCdcL

b) 
target/release/robonomics pair listen --peer QmUnozNz2tmzDeckoShMHbvGp1yWi4BKBiudkELwoKCdcL
- it provides robonomics node peer ID, i.e.: 12D3KooWATu4x31L1Vsje1Fx3GgA9Pddvhk6pVJH2ydE5enefg1F
- take it as arg to connect it:
target/release/robonomics-request-response-example /ip4/127.0.0.1/tcp/61241 12D3KooWATu4x31L1Vsje1Fx3GgA9Pddvhk6pVJH2ydE5enefg1F
