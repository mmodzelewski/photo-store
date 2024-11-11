run-desktop:
    cd clients && cargo tauri dev

run-backend:
    cargo run -p backend

run-web:
    cd web && pnpm dev

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

sort-deps:
    cargo sort --workspace

