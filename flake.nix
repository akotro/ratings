{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      gitignore,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.lib.${system};

        sqlFilter = path: _type: null != builtins.match ".*sql$" path;
        sqlOrCargo = path: type: (sqlFilter path type) || (craneLib.filterCargoSources path type);
        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = sqlOrCargo;
        };

        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [
          perl
          openssl
          mariadb
        ];

        commonArgs = {
          inherit src nativeBuildInputs buildInputs;
          pname = "ratings";
          version = "0.1.0";
          doCheck = false;
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        ratings = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;

            nativeBuildInputs = (commonArgs.nativeBuildInputs or [ ]) ++ [
              pkgs.sqlx-cli
            ];

            preBuild = ''
              export SQLX_OFFLINE_DIR=${craneLib.path ./.sqlx}
            '';
          }
        );

        frontend = pkgs.buildNpmPackage {
          name = "frontend";
          src = gitignore.lib.gitignoreSource ./frontend;

          npmDepsHash = "sha256-Rj/eQilByhhhTZvkxpzXx77uKZPuVnArMavL2b6pAVs=";

          installPhase = ''
            runHook preInstall

            mkdir -p $out
            cp -r build/* $out/

            runHook postInstall
          '';
        };

        full = pkgs.stdenv.mkDerivation {
          name = "ratings";
          src = gitignore.lib.gitignoreSource ./.;
          buildInputs = [
            ratings
            frontend
          ];

          buildPhase = "true";

          installPhase = ''
            mkdir -p $out/backend $out/frontend
            cp -r ${ratings}/* $out/backend/
            cp -r ${frontend}/* $out/frontend/
          '';
        };
      in
      {
        checks = {
          inherit ratings frontend;
          default = full;
        };

        packages = {
          inherit ratings frontend;

          default = full;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.nil
            pkgs.node2nix
            pkgs.sqlx-cli
            pkgs.libmysqlclient.dev
          ];
        };
      }
    );
}
