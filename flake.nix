{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        stdenv = pkgs.stdenv;
        lib = pkgs.lib;
      in
      rec {

        # `nix fmt'
        formatter = pkgs.nixpkgs-fmt;

        defaultPackage = packages.crab_share;

        packages = flake-utils.lib.flattenTree rec {
          crab_share = pkgs.rustPlatform.buildRustPackage {
            pname = "crab_share";
            version = "0.1.0";
            src = self;
            cargoLock = { lockFile = ./Cargo.lock; };
            nativeBuildInputs = with pkgs;
              lib.optionals stdenv.isLinux [ pkg-config ];
            buildInputs = with pkgs; [ openssl ];
          };
        };

      });
}
