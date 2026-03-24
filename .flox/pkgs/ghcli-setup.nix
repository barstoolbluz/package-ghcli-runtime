{ rustPlatform, lib, pkg-config }:

rustPlatform.buildRustPackage {
  pname = "ghcli-setup";
  version = "0.2.0";

  src = ../../src/ghcli-setup;

  cargoLock.lockFile = ../../src/ghcli-setup/Cargo.lock;

  nativeBuildInputs = [ pkg-config ];

  postInstall = ''
    mkdir -p $out/share/man/man1
    $out/bin/ghcli-mangen $out/share/man/man1
    rm -f $out/bin/ghcli-mangen
  '';

  meta = with lib; {
    description = "Flox GitHub setup wizard (Rust)";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
