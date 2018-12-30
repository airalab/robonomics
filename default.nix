{ stdenv
, fetchFromGitHub
, rustPlatform
, pkgconfig
, openssl
}:

rustPlatform.buildRustPackage rec {
  name = "robonomics-${version}";
  version = "0.0.0";

  src = ./.; 

  cargoSha256 = "00wkaxqj2v5zach5xcqfzf6prc0gxy2v47janglp44xbxbx9xk08";

  buildInputs = [ pkgconfig openssl ];

  meta = with stdenv.lib; {
    description = "Robonomics Node Implementation";
    homepage = http://robonomics.network;
    license = licenses.bsd3;
    maintainers = [ maintainers.akru ];
    platforms = platforms.linux;
  };
}
