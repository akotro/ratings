backend-build:
    cargo build
    cargo sqlx prepare -- --all-targets

backend-run:
    cargo run

frontend-build:
    cd ./frontend && npm run check && npm run build

frontend-run:
    cd ./frontend && npm run dev

test:
    cargo test

# Root has no password, matching the dev shell's DATABASE_URL. Stops with the recipe.
[doc("local MariaDB in ./.mysql, with the database created and migrated")]
up:
    #!/usr/bin/env bash
    set -euo pipefail
    [ -d .mysql ] || mariadb-install-db --datadir="$PWD/.mysql" \
      --auth-root-authentication-method=normal >/dev/null
    mariadbd --datadir="$PWD/.mysql" --socket="$PWD/.mysql/mysqld.sock" --bind-address=127.0.0.1 &
    db_pid=$!
    trap 'kill $db_pid' EXIT
    until mariadb-admin --socket="$PWD/.mysql/mysqld.sock" ping >/dev/null 2>&1; do sleep 0.5; done
    sqlx database create
    sqlx migrate run
    wait $db_pid

database *args:
    cargo sqlx database {{ args }}

migrate *args:
    cargo sqlx migrate {{ args }}

mariadb *args:
    mariadb -h 127.0.0.1 -u root ratings {{ args }}
