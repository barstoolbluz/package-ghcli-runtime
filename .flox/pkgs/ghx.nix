{ stdenv, lib, makeWrapper, gh, jq, gum, less, coreutils }:

stdenv.mkDerivation {
  pname = "ghx";
  version = "0.1.0";

  src = ../../scripts;

  nativeBuildInputs = [ makeWrapper ];

  dontBuild = true;

  installPhase = ''
    mkdir -p $out/bin $out/lib/ghx/{lib,commands,learn}

    install -m 0755 ghx $out/bin/ghx
    install -m 0755 ghcli-runtime.sh $out/bin/ghcli-runtime

    install -m 0644 lib/ghx-common.sh $out/lib/ghx/lib/

    for f in commands/*.sh; do
      install -m 0644 "$f" $out/lib/ghx/commands/
    done

    for f in learn/*.md; do
      install -m 0644 "$f" $out/lib/ghx/learn/
    done

    wrapProgram $out/bin/ghx \
      --prefix PATH : ${lib.makeBinPath [ gh jq gum less coreutils ]}
  '';

  meta = with lib; {
    description = "Educational GitHub CLI wrapper with built-in learning";
    license = licenses.mit;
    platforms = platforms.unix;
  };
}
