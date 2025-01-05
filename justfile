run-desktop:
    cd clients && cargo tauri dev

run-desktop-release:
    cd clients && cargo tauri dev --release

run-backend:
    cargo run -p backend

reset-db:
    cd backend && sqlx database reset --source db/migrations

run-web:
    cd web && pnpm dev

fmt:
    cargo fmt --all

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

sort-deps:
    cargo sort --workspace

