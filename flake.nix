{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      inherit (pkgs) lib;

      craneLib = crane.lib.${system};

      sqlFilter = path: _type: null != builtins.match ".*sql$" path;
      sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);
      src = lib.cleanSourceWith {
        src = craneLib.path ./.;
        filter = sqlOrCargo;
      };

      nativeBuildInputs = with pkgs; [pkg-config];
      buildInputs = with pkgs; [perl openssl mariadb];

      commonArgs = {
        inherit src nativeBuildInputs buildInputs;
        pname = "ratings";
        version = "0.1.0";
        # strictDeps = true;
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      ratings = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;

          nativeBuildInputs =
            (commonArgs.nativeBuildInputs or [])
            ++ [
              pkgs.sqlx-cli
            ];

          preBuild = ''
            export SQLX_OFFLINE_DIR=${craneLib.path ./.sqlx}
          '';
        });
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit ratings;
      };
      packages = {
        default = ratings;
        inherit ratings;
      };
      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [
          pkgs.nil
          pkgs.sqlx-cli
          pkgs.libmysqlclient.dev
        ];
      };
    });
}
