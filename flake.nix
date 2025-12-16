{
  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { self, ... }:
      {
        imports = [
          inputs.devenv.flakeModule
        ];

        systems = [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ];

        perSystem =
          {
            config,
            self',
            inputs',
            pkgs,
            system,
            ...
          }:
          let
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ (import inputs.rust-overlay) ];
            };
            inherit (pkgs) lib;
            rust = pkgs.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            };
            craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (_: rust);

            unfilteredRoot = ./.;
            src = lib.fileset.toSource {
              root = unfilteredRoot;
              fileset = lib.fileset.unions [
                (craneLib.fileset.commonCargoSources unfilteredRoot)
                ./migrations
                ./.sqlx
              ];
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
                  export SQLX_OFFLINE=true
                  export SQLX_OFFLINE_DIR=${craneLib.path ./.sqlx}
                '';
              }
            );

            frontend = pkgs.buildNpmPackage {
              name = "frontend";
              src = ./frontend;

              npmDepsHash = "sha256-yZ6fjXHaho0rfwItUD3Op/O3J9yL8Zs1xt9NQNMXuXM=";

              installPhase = ''
                runHook preInstall

                mkdir -p $out
                cp -r build/* $out/

                runHook postInstall
              '';
            };

            full = pkgs.stdenv.mkDerivation {
              name = "ratings";
              src = ./.;
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
            formatter = pkgs.nixfmt-tree;

            checks = {
              inherit ratings frontend;
              default = full;
            };

            packages = {
              inherit ratings frontend;

              default = full;
            };

            devenv.shells.default = {
              # https://devenv.sh/reference/options/
              packages = [
                pkgs.pkg-config
                pkgs.openssl
                pkgs.gdb
                pkgs.lldb
                pkgs.vscode-extensions.vadimcn.vscode-lldb

                rust
                pkgs.rust-bin.stable.latest.rust-analyzer
                pkgs.cargo-edit
                pkgs.cargo-machete
                pkgs.cargo-outdated
                pkgs.sqlx-cli
                pkgs.libmysqlclient.dev

                pkgs.nodejs
                pkgs.svelte-language-server
                pkgs.tailwindcss-language-server
                pkgs.typescript-language-server
                pkgs.vtsls
              ];

              env = {
                DATABASE_URL = "mysql://local:@127.0.0.1:3306/ratings";
              };

              services = {
                mysql = {
                  enable = true;
                  package = pkgs.mariadb;
                  settings.mysqld = {
                    bind-address = "127.0.0.1";
                  };
                  initialDatabases = [
                    { name = "ratings"; }
                  ];
                  ensureUsers = [
                    {
                      name = "local";
                      ensurePermissions = {
                        "*.*" = "ALL PRIVILEGES";
                      };
                    }
                  ];
                };
              };

              processes = {
                mysql-configure.process-compose = {
                  ready_log_line = "Adding user: local";
                };
                migrate = {
                  exec = # bash
                    ''
                      echo "Running migrations..."
                      ${lib.getExe pkgs.sqlx-cli} migrate run
                    '';
                  process-compose = {
                    depends_on = {
                      mysql-configure = {
                        condition = "process_log_ready";
                      };
                    };
                  };
                };
              };
            };
          };
      }
    );
}
