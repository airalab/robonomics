1. Generate 2 private.pk8 keys (see link.txt): 
first one for server 
second for client

From keys will be extracted peerId and they are static (not rendom from run to run).

2. Run two examples of robonomics from different folders  (or in different PCs, not tested, see note)
where place different keys:

- one instance of robonomics as reqres server from one drectory with server private.pk8 key:
$ ./robonomics pair listen --key Qma7vLWTmTnwcXfLF4iKEuJqvfCZGRrseYTuxk2GdVE9fZ

- other one instance of robonomics as reqres client from other directory with client private.pk8 key:
$ ../robonomics pair connect --key QmUnozNz2tmzDeckoShMHbvGp1yWi4BKBiudkELwoKCdcL

In case if reqres server mathes address from command line and peer trying to connect it continue communication.
In other case it reject connection (with message about saving unexpected peer Id).


Note:
adrees is hardcoded to connect with /ip4/127.0.0.1/tcp/61241, while no discovery at present time
