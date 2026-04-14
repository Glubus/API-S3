# Architecture

## Overview

```
HTTP Request
     │
     ▼
┌─────────────┐
│   Router    │  Axum routes defined in src/routes/
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Controllers │  Request parsing, validation (src/controllers/)
└──────┬──────┘
       │
       ├──────────────────────┐
       ▼                      ▼
┌─────────────┐     ┌──────────────────┐
│   Storage   │     │ Image Processor  │
│   Service   │     │   (pipeline)     │
└──────┬──────┘     └────────┬─────────┘
       │                     │
       │            ┌────────┴──────────────┐
       │            ▼         ▼             ▼
       │         Decoder   Resizer      Encoder
       │        (zune-jpeg) (fast_image  (webp /
       │                    _resize)     image)
       ▼
  S3 / MinIO
  or local FS
```

## Modules

### `src/routes/`
Axum route registration. Maps HTTP paths to controller functions.

### `src/controllers/`
- `resources/get.rs`: single image request handler
- `resources/bulk.rs`: batch request handler (streams `multipart/mixed`)

### `src/services/`
- `storage.rs`: `StorageService`: abstraction over S3 SDK and local filesystem
- `image_processor/`: three-stage pipeline:
  - `decoder.rs`: JPEG → raw pixels (zune-jpeg)
  - `resizer.rs`: pixel resampling (fast_image_resize, SIMD)
  - `encoder.rs`: raw pixels → WebP (libwebp FFI via `webp` crate)
- `cache.rs`: in-memory LRU cache for processed images
- `multipart.rs`: `MultipartBuilder`: constructs streaming `multipart/mixed` responses
- `telemetry.rs`: OpenTelemetry SDK initialization (OTLP gRPC exporter)
- `errors.rs`: typed error enum with `thiserror`

### `src/models/`
- `resources.rs`: request/response types
- `device.rs`: `DeviceConfig` (Phone / Desktop targeting)

## Request Flow (single image)

1. `GET /r/{category}/{width}/{target}/{*file_path}`
2. Router → `get_resource` controller
3. Validate `width` (> 0, ≤ 4000), sanitize path (no `..`)
4. Check in-memory cache → return cached bytes if hit
5. `StorageService::get()` → fetch raw bytes from S3 or disk
6. `ImageProcessor::process()`:
   a. Decode JPEG → `Vec<u8>` pixel buffer
   b. Resize to requested width (maintain aspect ratio)
   c. Encode to WebP
7. Store result in cache
8. Return `image/webp` response

## Bulk Request Flow

1. `POST /r/bulk` with JSON body `[{path, width, target}, …]`
2. Spawn one Tokio task per image (bounded concurrency via channel)
3. Each task runs the same pipeline as above
4. Results streamed back as `multipart/mixed` parts via `tokio-stream`
