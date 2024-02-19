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

  outputs = {
    self,
    nixpkgs,
    gitignore,
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
        doCheck = false;
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

      frontendNodeDependencies = (pkgs.callPackage ./frontend/default.nix {}).nodeDependencies;
      frontend = pkgs.stdenv.mkDerivation {
        name = "frontend";
        src = gitignore.lib.gitignoreSource ./frontend;
        buildInputs = [pkgs.nodejs];
        buildPhase = ''
          ln -s ${frontendNodeDependencies}/lib/node_modules ./node_modules
          export PATH="${frontendNodeDependencies}/bin:$PATH"

          npm run build

          mkdir -p $out/build
          cp -r build $out/

          cp -r ./package*.json $out/
          cp -r ./node_modules $out/
        '';
      };
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit ratings;
      };
      packages = {
        inherit ratings frontend;

        default = pkgs.stdenv.mkDerivation {
          name = "ratings";
          src = gitignore.lib.gitignoreSource ./.;
          buildInputs = [ratings frontend];

          buildPhase = "true";

          installPhase = ''
            mkdir -p $out/backend $out/frontend
            cp -r ${ratings}/* $out/backend/
            cp -r ${frontend}/* $out/frontend/
          '';
        };
      };
      # devShells.default = craneLib.devShell {
      # checks = self.checks.${system};
      # Additional dev-shell environment variables can be set directly
      # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

      # RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

      devShells.default = pkgs.mkShell {
        packages = [
          pkgs.nil
          pkgs.sqlx-cli
          pkgs.libmysqlclient.dev
        ];
      };
    });
}
