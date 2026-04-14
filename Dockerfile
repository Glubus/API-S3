# Builder stage
FROM rust:1.93-trixie AS chef
USER root
RUN apt-get update -qq && apt-get install -y -qq mold && cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin api_s3

# Runtime stage
FROM debian:trixie-slim AS runtime
RUN groupadd -r appuser && useradd -r -g appuser appuser
WORKDIR /app
RUN apt-get update && apt-get upgrade -y && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder --chown=appuser:appuser /app/target/release/api_s3 /app/api_s3
USER appuser
ENV RUST_LOG=info
EXPOSE 3000
CMD ["./api_s3"]
