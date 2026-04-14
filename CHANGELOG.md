# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-14

### Added

- MinIO / S3 storage backend via `aws-sdk-s3`
- Axum HTTP server with multipart support
- Image processing pipeline: JPEG decode (zune-jpeg), resize (fast_image_resize), WebP encode (libwebp FFI)
- Resource endpoints:
  - `GET /r/{category}/{width}/{target}/{*file_path}`: serve resized image
  - `POST /r/bulk`: batch resize with `multipart/mixed` streaming response
- `DeviceConfig` model for device-specific targeting (Phone, Desktop)
- StorageService abstraction layer (S3 or local filesystem fallback)
- Security: max width cap (4000 px), path traversal prevention
- OpenTelemetry / Jaeger integration via OTLP gRPC
- Kubernetes manifests (Kustomize overlays for production and pre-prod)
- GitLab CI pipeline: build → quality (lint, test, fmt) → package → deploy
- OpenAPI documentation served at `/docs` via utoipa + Scalar
- mimalloc high-performance allocator
- Criterion benchmarks for image processing

### Fixed

- Width validation: reject `width=0`
- Clippy pedantic warnings resolved
- Non-root container execution (`appuser`)
- Dockerfile base image pinned to `debian:trixie-slim`

[1.0.0]: https://github.com/your-org/api-s3/releases/tag/v1.0.0
