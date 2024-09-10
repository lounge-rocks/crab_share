{

  inputs = { nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; };

  outputs = { self, nixpkgs, ... }:
    let
      # System types to support.
      supportedSystems = [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];

      # Helper function to generate an attrset '{ x86_64-linux = f "x86_64-linux"; ... }'.
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      # Nixpkgs instantiated for supported system types.
      nixpkgsFor = forAllSystems (system:
        import nixpkgs { inherit system; overlays = [ self.overlay ]; });
    in
    {

      formatter = forAllSystems (system: nixpkgsFor.${system}.nixpkgs-fmt);

      overlay = final: prev:
        let
          pkgs = nixpkgsFor.${prev.system};
          inherit (pkgs) lib;
        in
        {
          crab_share = let manifest = (lib.importTOML ./Cargo.toml).package; in
            pkgs.rustPlatform.buildRustPackage {
              pname = manifest.name;
              version = manifest.version;
              src = lib.cleanSource self;
              cargoLock = { lockFile = ./Cargo.lock; };
              nativeBuildInputs = with pkgs; lib.optionals stdenv.isLinux [ pkg-config ];
              buildInputs = with pkgs; [ openssl ]; # TODO: check how to make this work on darwin?
            };

        };

      devShells = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [ cargo openssl ];
          };
        });

      packages = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
          default = pkgs.crab_share;
          crab_share = pkgs.crab_share;
        });

    };

}
