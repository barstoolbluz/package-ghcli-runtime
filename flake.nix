{
  description = "ghx: GitHub CLI setup and reset tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          ghcli-setup = pkgs.callPackage ./.flox/pkgs/ghcli-setup.nix {};
          ghx = pkgs.callPackage ./.flox/pkgs/ghx.nix { inherit ghcli-setup; };
        in {
          ghcli-setup = ghcli-setup;
          ghx = ghx;
          default = ghx;
        }
      );
    };
}
