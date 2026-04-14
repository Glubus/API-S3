# Getting Started

## What is api-s3?

`api-s3` is a high-performance Rust image processing microservice built with [Axum](https://github.com/tokio-rs/axum). It accepts image requests, fetches the source from S3/MinIO (or local disk), resizes them on-the-fly using SIMD-accelerated processing, and returns WebP-encoded output, all in a single HTTP request.

Key properties:
- **Fast**: zune-jpeg decode + fast_image_resize (SIMD) + libwebp encode
- **Memory-efficient**: streaming `multipart/mixed` responses for bulk requests
- **Observable**: OpenTelemetry traces exported via OTLP gRPC
- **Secure**: max width cap (4000 px), path traversal prevention, non-root container

## Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.85+ (edition 2024) |
| just | any recent version |
| Docker | 20+ (for containerized setup) |
| MinIO or S3 | optional (local filesystem fallback available) |

Install `just`:
```bash
cargo install just
# or: brew install just / pacman -S just
```

## Quick Start (local, no S3)

```bash
# 1. Clone the repo
git clone https://github.com/your-org/api-s3.git
cd api-s3

# 2. Copy and edit environment config
cp .env.example .env
# USE_S3=false by default, local assets/ directory used

# 3. Run
cargo run --bin api_s3
```

Server starts on `http://localhost:3000`.

## Quick Start (with MinIO)

```bash
# Start MinIO locally
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minio-admin \
  -e MINIO_ROOT_PASSWORD=minio-admin \
  quay.io/minio/minio server /data --console-address ":9001"

# Configure .env
USE_S3=true
S3_ENDPOINT=http://localhost:9000
S3_ACCESS_KEY=minio-admin
S3_SECRET_KEY=minio-admin
S3_BUCKET=images
S3_REGION=us-east-1

# Run
cargo run --bin api_s3
```

## Development Commands

```bash
just            # list all commands
just check      # clippy (pedantic + warnings as errors)
just test       # cargo test
just fmt        # cargo fmt
just fmt-check  # check formatting without modifying files
just audit      # fmt-check + check + test (full pre-commit suite)
```

## API

See the interactive OpenAPI docs at `http://localhost:3000/docs` (Scalar UI).

Main endpoints:

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/r/{category}/{width}/{target}/{*file_path}` | Fetch + resize + encode single image |
| `POST` | `/r/bulk` | Batch resize, returns `multipart/mixed` stream |

See [api.md](./api.md) for full parameter reference.
