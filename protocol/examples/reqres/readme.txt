Request-response 

Works for two different hosts, client and server:
 
To start as a server run with host interface multiaddr to bind:
 robonomics-request-response-example local_multiaddr

as a client:
 robonomics-request-response-example remote_multiaddr server_peerID

For example server:
 robonomics-request-response-example /ip4/192.168.1.6/tcp/61241 

client with PeerID as additional argument from printf() of server application to stdout, i.e.:
 robonomics-request-response-example /ip4/192.168.1.6/tcp/61241 Qma7vLWTmTnwcXfLF4iKEuJqvfCZGRrseYTuxk2GdVE9fZ

Note:  
To have stable static PeerID instead of randomly generated need to create keypairs file:
ref. https://docs.rs/libp2p/0.12.0/libp2p/identity/enum.Keypair.html

openssl genrsa -out private.pem 2048
openssl pkcs8 -in private.pem -inform PEM -topk8 -out private.pk8 -outform DER -nocrypt
rm private.pem   # optional
