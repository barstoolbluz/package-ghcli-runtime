{ stdenv, lib, ghcli-setup }:

let
  buildMeta = builtins.fromJSON (builtins.readFile ../../build-meta/ghx.json);
  baseVersion = "0.9.1";
  version = "${baseVersion}+${buildMeta.git_rev_short}";
in

stdenv.mkDerivation {
  pname = "ghx";
  inherit version;

  dontUnpack = true;
  dontBuild = true;

  installPhase = ''
    mkdir -p $out/bin
    install -m 0755 ${ghcli-setup}/bin/ghcli-setup $out/bin/ghcli-setup
    install -m 0755 ${ghcli-setup}/bin/ghcli-reset $out/bin/ghcli-reset
  '';

  meta = with lib; {
    description = "GitHub CLI setup and reset tools";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
