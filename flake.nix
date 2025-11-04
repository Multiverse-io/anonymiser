{
  description = "A PostgreSQL anonymisation CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [rust-overlay.overlays.default];
        pkgs = import nixpkgs {inherit overlays system;};

        rust = pkgs.rust-bin.stable.latest.default.override {extensions = ["rust-src"];};
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        # Rust with musl target for static builds on Linux
        rustWithMusl = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src"];
          targets = ["x86_64-unknown-linux-musl"];
        };

        rustPlatformMusl = pkgs.makeRustPlatform {
          cargo = rustWithMusl;
          rustc = rustWithMusl;
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
              cmake
              perl # Required for vendored OpenSSL build
            ];
            # Run-time dependencies.
            buildInputs = with pkgs;
              [
                openssl
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
                with pkgs.darwin.apple_sdk.frameworks; [
                  Security
                  SystemConfiguration
                ]
              );

            checkFlags = [
              # Skip tests which require access to a PostgreSQL server.
              "--skip=anonymiser::tests::successfully_transforms"
              "--skip=anonymiser::tests::successfully_truncates"
              "--skip=parsers::db_schema::tests::can_read_db_columns"
            ];
          };

          # Static musl build for Linux distribution
          anonymiser-musl = rustPlatformMusl.buildRustPackage {
            pname = "${manifest.name}-musl";
            version = manifest.version;
            src = pkgs.nix-gitignore.gitignoreSource [] ./.;
            cargoLock.lockFile = ./Cargo.lock;

            # Target musl for static linking
            CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";

            # Compile-time dependencies.
            nativeBuildInputs = with pkgs; [
              pkg-config
              cmake
              perl # Required for vendored OpenSSL build
            ];

            # With vendored OpenSSL and static linking, we don't need runtime dependencies
            buildInputs = [];

            checkFlags = [
              # Skip tests which require access to a PostgreSQL server.
              "--skip=anonymiser::tests::successfully_transforms"
              "--skip=anonymiser::tests::successfully_truncates"
              "--skip=parsers::db_schema::tests::can_read_db_columns"
            ];

            # Only build on Linux
            meta.platforms = ["x86_64-linux"];
          };

          default = self.packages.${system}.anonymiser;
        };
      }
    );
}
