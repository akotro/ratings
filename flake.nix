{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { self, ... }:
      {
        systems = [
          "x86_64-linux"
          "aarch64-linux"
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
                ./fixtures
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

            # `#[sqlx::test]` needs a live server, so the tests are only compiled here and run in
            # `vm-test`.
            ratings-tests = craneLib.mkCargoDerivation (
              commonArgs
              // {
                inherit cargoArtifacts;
                pnameSuffix = "-tests";

                nativeBuildInputs = (commonArgs.nativeBuildInputs or [ ]) ++ [ pkgs.jq ];

                SQLX_OFFLINE = "true";
                doInstallCargoArtifacts = false;

                buildPhaseCargoCommand = ''
                  cargo test --release --locked --no-run --message-format=json-render-diagnostics \
                    | jq -r 'select(.profile.test == true) | .executable' > test-executables
                '';

                installPhaseCommand = ''
                  mkdir -p $out/bin
                  xargs -I{} install -m755 {} $out/bin < test-executables
                '';
              }
            );

            frontend = pkgs.buildNpmPackage {
              name = "frontend";
              src = ./frontend;

              npmDepsHash = "sha256-TpVwps5Qs7U5yt/KWFWmyB59sZ+tGd0gSAMMBVqTuyM=";

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
              # What `just test` runs, minus doctests, which cannot be run from a prebuilt binary.
              vm-test = pkgs.testers.nixosTest {
                name = "ratings-tests";

                nodes.machine = {
                  services.mysql = {
                    enable = true;
                    package = pkgs.mariadb;
                    ensureDatabases = [ "ratings" ];
                    # sqlx cannot do `unix_socket` auth, which `ensureUsers` sets up. Global
                    # privileges because `#[sqlx::test]` creates a database per test.
                    initialScript = pkgs.writeText "ratings-test-user.sql" ''
                      CREATE USER 'ratings'@'localhost' IDENTIFIED BY 'ratings';
                      GRANT ALL PRIVILEGES ON *.* TO 'ratings'@'localhost';
                    '';
                  };
                };

                testScript = ''
                  machine.wait_for_unit("mysql.service")
                  # TCP, since over the socket sqlx fails closing its setup connection (EPIPE).
                  machine.wait_for_open_port(3306)
                  machine.succeed(
                      "export DATABASE_URL=mysql://ratings:ratings@127.0.0.1/ratings "
                      "&& for t in ${ratings-tests}/bin/*; do $t || exit 1; done",
                      timeout=900,
                  )
                '';
              };
              default = full;
            };

            packages = {
              inherit ratings frontend;
              default = full;
            };

            devShells.default = pkgs.mkShell {
              packages = [
                pkgs.just
                pkgs.just-lsp
                pkgs.nix-fast-build
                pkgs.nix-output-monitor

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
                pkgs.mariadb # `just up`

                pkgs.nodejs
                pkgs.svelte-language-server
                pkgs.tailwindcss-language-server
                pkgs.typescript-language-server
                pkgs.vtsls

                pkgs.treefmt
                pkgs.taplo
              ];

              CODELLDB_PATH = "${pkgs.vscode-extensions.vadimcn.vscode-lldb}/share/vscode/extensions/vadimcn.vscode-lldb/adapter/codelldb";
              DATABASE_URL = "mysql://root@127.0.0.1:3306/ratings";
            };
          };
      }
    );
}
