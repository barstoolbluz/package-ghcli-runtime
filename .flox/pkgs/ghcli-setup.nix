{ rustPlatform, lib, pkg-config, stdenv, darwin }:

rustPlatform.buildRustPackage {
  pname = "ghcli-setup";
  version = "0.2.0";

  src = ../../src/ghcli-setup;

  cargoLock.lockFile = ../../src/ghcli-setup/Cargo.lock;

  nativeBuildInputs = [ pkg-config ];

  buildInputs = lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
  ];

  meta = with lib; {
    description = "Flox GitHub setup wizard (Rust)";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
