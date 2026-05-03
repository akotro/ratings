backend-build:
    cargo build

backend-run:
    cargo run

frontend-build:
    cd ./frontend && npm run check && npm run build

frontend-run:
    cd ./frontend && npm run dev

test:
    cargo test

up:
    devenv up -D

attach:
    process-compose attach

down:
    process-compose down

database *args:
    cargo sqlx database {{ args }}

migrate *args:
    cargo sqlx migrate {{ args }}

mariadb *args:
    mariadb -u root ratings {{ args }}
