# with import <nixpkgs> {}; mkShellNoCC {
#   nativeBuildInputs = [
#     fswatch
#     hugo
#     nodePackages.autoprefixer
#     nodePackages.postcss-cli
#   ];
# }

# { pkgs ? import <nixpkgs> {} }:

# pkgs.firefox

{
  description = "packages for a development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-22.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    let
      systems = ["x86_64-linux" "aarch64-linux"];
    in
    flake-utils.lib.eachSystem systems (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages = {
          helix = pkgs.helix;

          # devShell c
        };

        # defaultPackage = with pkgs; [helix];
      }
    );
}


