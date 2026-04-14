# api-s3

A high-performance image resizing microservice written in Rust. Fetches images from S3/MinIO (or local disk), resizes on-the-fly using SIMD-accelerated processing, and returns WebP output, all in a single HTTP round-trip.

## Background

The problem: serving raw images directly from S3 means zero optimization. The frontend downloads 4K originals for a phone screen, every time.

The solution: a Rust microservice that sits between the client and MinIO. The client requests an image with a target width and device type. The service fetches the source from S3, resizes it, encodes to WebP, caches the result in memory, and responds. Subsequent requests for the same parameters are served from cache.

```
Client → GET /r/images/400/phone/photo.jpg
              ↓ cache miss
         Fetch from MinIO → Decode → Resize (SIMD) → Encode WebP → Cache → 200 OK
              ↓ cache hit
         200 OK (from memory)
```

Every technical decision was driven by benchmarks:

| Decision | Before | After | Gain |
|----------|--------|-------|------|
| Resize: Lanczos3 SIMD (`fast_image_resize`) vs standard | 29 ms P95 | <1 ms | ×29 |
| Decoder: `zune-jpeg` (pure Rust, no libjpeg) | - | ~17 ms | 0 C deps |
| WebP pipeline: `libwebp` decode + `method=0` encode | 17 ms / 14.5 ms | 6.8 ms / 3.6 ms | −60% / −75% |
| Output size | 100 KB | 75 KB | −25% |

Device-based quality is decided server-side (`Phone` → 75%, `Desktop` → 85%). No client-controlled quality parameter.

## Why Rust?

Most image resizing services (imgproxy, thumbor, Imagor) are written in Go or Python. This one is Rust:

- **Faster decode**: [zune-jpeg](https://github.com/etemesi254/zune-image): pure Rust, no libjpeg, no C build deps
- **SIMD resize**: [fast_image_resize](https://github.com/cykooz/fast_image_resize) with SSE4.1/AVX2/AVX-512/NEON: Lanczos3 quality at Nearest Neighbor cost
- **Native WebP**: libwebp FFI via the [`webp`](https://crates.io/crates/webp) crate
- **Tiny footprint**: ~10 MB Docker image, ~20 MB RSS at idle

## Benchmarks

> Measured on Intel i7-13620H, 64 GB RAM (`cargo bench`)

_Results coming soon. Run `cargo bench` on your machine and open a PR!_

## Features

- Single image endpoint: fetch → resize → WebP in one request
- Bulk endpoint: batch processing with streaming `multipart/mixed` response
- S3/MinIO backend with local filesystem fallback
- Device targeting (`Phone` / `Desktop` presets)
- Path traversal prevention, max width cap (4000 px)
- OpenTelemetry traces via OTLP gRPC (Jaeger, Tempo, …)
- Kubernetes-ready: Kustomize overlays for production + pre-prod
- GitLab CI pipeline (build → lint → test → package → deploy)
- OpenAPI docs at `/docs` (Scalar UI)

## Quick Start

**Requirements:** Rust 1.85+, [`just`](https://github.com/casey/just)

```bash
git clone https://github.com/your-org/api-s3.git
cd api-s3
cp .env.example .env
cargo run --bin api_s3
# → http://localhost:3000
```

Interactive API docs: `http://localhost:3000/docs`

## API

### Single image

```
GET /r/{category}/{width}/{target}/{*file_path}
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `category` | string | Image category (maps to S3 prefix or local subdirectory) |
| `width` | u32 | Target width in pixels (1–4000) |
| `target` | string | Device target (`phone` or `desktop`) |
| `file_path` | string | Relative path to the image file |

Returns: `image/webp`

### Bulk

```
POST /r/bulk
Content-Type: application/json

[
  { "path": "products/shoes.jpg", "width": 400, "target": "phone" },
  { "path": "products/bag.jpg",   "width": 800, "target": "desktop" }
]
```

Returns: `multipart/mixed` stream, one WebP part per image, in order.

## Configuration

Copy `.env.example` to `.env`:

```env
RUST_LOG=info

# Storage
USE_S3=false                          # true = S3/MinIO, false = local disk
IMAGE_DIR=assets/images               # used when USE_S3=false
S3_ENDPOINT=http://localhost:9000
S3_ACCESS_KEY=minio-admin
S3_SECRET_KEY=minio-admin
S3_BUCKET=images
S3_REGION=us-east-1

# Observability
OTEL_EXPORTER_OTLP_ENDPOINT=http://your-super-jaeger.net:4317
OTEL_SERVICE_NAME=api-s3
```

Full reference: [docs/configuration.md](docs/configuration.md)

## Development

```bash
just              # list commands
just audit        # fmt-check + clippy (pedantic) + tests (same as CI)
just fmt          # auto-format
cargo bench       # run image processing benchmarks
```

Pre-commit hook (requires [`pre-commit`](https://pre-commit.com)):

```bash
pip install pre-commit
pre-commit install
# runs `just audit` on every commit
```

## Deployment

Docker:

```bash
docker build -t api-s3 .
docker run -p 3000:3000 --env-file .env api-s3
```

Kubernetes (Kustomize):

```bash
kubectl apply -k k8s/overlays/production
```

See [docs/deployment.md](docs/deployment.md) for full CI/CD setup and required variables.

## Architecture

```
Request → Router → Controller → StorageService → ImageProcessor → Response
                                    │                  │
                                 S3/MinIO           Decode (zune-jpeg)
                                 or disk            Resize (fast_image_resize)
                                                    Encode (libwebp)
```

Full breakdown: [docs/architecture.md](docs/architecture.md)

## License

MIT
