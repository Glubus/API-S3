# Justfile for Api-S3

# List available commands
default:
    @just --list

# Run cargo check with strict settings
check:
    cargo clippy -- -D clippy::pedantic -D warnings

# Run tests
test:
    cargo test

# Format code
fmt:
    cargo fmt

# Check formatting without changing files
fmt-check:
    cargo fmt -- --check

# Run full audit (format check, strict clippy, tests)
audit: fmt-check check test
    @echo "Audit complete!"
