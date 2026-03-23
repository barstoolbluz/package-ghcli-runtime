{ stdenv, lib, makeWrapper, gh, jq, gum, less, coreutils, ghcli-setup }:

let
  buildMeta = builtins.fromJSON (builtins.readFile ../../build-meta/ghx.json);
  baseVersion = "0.6.9";
  version = "${baseVersion}+${buildMeta.git_rev_short}";
in

stdenv.mkDerivation {
  pname = "ghx";
  inherit version;

  src = ../../scripts;

  nativeBuildInputs = [ makeWrapper ];

  dontBuild = true;

  installPhase = ''
    mkdir -p $out/bin $out/lib/ghx/{lib,commands,learn}

    install -m 0755 ghx $out/bin/ghx
    install -m 0755 ${ghcli-setup}/bin/ghcli-setup $out/bin/ghcli-setup

    install -m 0644 lib/ghx-common.sh $out/lib/ghx/lib/

    for f in commands/*.sh; do
      install -m 0644 "$f" $out/lib/ghx/commands/
    done

    for f in learn/*.md; do
      install -m 0644 "$f" $out/lib/ghx/learn/
    done

    # Patch version string to include git rev
    substituteInPlace $out/lib/ghx/lib/ghx-common.sh \
      --replace-fail 'GHX_VERSION="0.1.0"' 'GHX_VERSION="${version}"'

    wrapProgram $out/bin/ghx \
      --prefix PATH : ${lib.makeBinPath [ gh jq gum less coreutils ]}
  '';

  meta = with lib; {
    description = "Educational GitHub CLI wrapper with built-in learning";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
