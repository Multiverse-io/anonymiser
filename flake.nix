{
  description = "A PostgreSQL anonymisation CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [rust-overlay.overlays.default];
      pkgs = import nixpkgs {inherit overlays system;};

      rust = pkgs.rust-bin.stable.latest.default.override {extensions = ["rust-src"];};
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rust;
        rustc = rust;
      };

      manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
    in {
      # `nix develop`.
      devShells = {
        default = pkgs.mkShell {
          inputsFrom = [self.packages.${system}.anonymiser];
          buildInputs = with pkgs; [rust-analyzer];
        };
      };

      # `nix fmt`.
      formatter = pkgs.alejandra;

      # `nix build`.
      packages = {
        anonymiser = rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          src = pkgs.nix-gitignore.gitignoreSource [] ./.;
          cargoLock.lockFile = ./Cargo.lock;

          # Compile-time dependencies.
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          # Run-time dependencies.
          buildInputs = with pkgs;
            [
              openssl
            ]
            ++ pkgs.lib.optional pkgs.stdenv.isDarwin pkgs.darwin.apple_sdk.frameworks.Security;

          checkFlags = [
            # Skip tests which require acces to a PostgreSQL server.
            "--skip=anonymiser::tests::successfully_transforms"
            "--skip=parsers::db_schema::tests::can_read_db_columns"
          ];
        };
        default = self.packages.${system}.anonymiser;
      };
    });
}
