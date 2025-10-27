run-desktop:
    cd clients/native && cargo tauri dev

run-desktop-release:
    cd clients/native && cargo tauri dev --release

run-android:
    cd clients/native && cargo tauri android dev

run-backend:
    cargo run -p backend

reset-db:
    cd backend && sqlx database reset --source db/migrations

run-web:
    cd clients/web && pnpm dev

fmt:
    cargo fmt --all

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

sort-deps:
    cargo sort --workspace

