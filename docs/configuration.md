# Configuration

All configuration is via environment variables. Copy `.env.example` to `.env` and adjust.

## Variables

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |

### Storage

| Variable | Default | Description |
|----------|---------|-------------|
| `USE_S3` | `false` | Set `true` to use S3/MinIO, `false` for local filesystem |
| `IMAGE_DIR` | `assets/images` | Local image directory (ignored when `USE_S3=true`) |
| `S3_ENDPOINT` | `http://localhost:9000` | S3/MinIO endpoint URL |
| `S3_ACCESS_KEY` |: | S3 access key |
| `S3_SECRET_KEY` |: | S3 secret key |
| `S3_BUCKET` | `images` | Bucket name |
| `S3_REGION` | `us-east-1` | AWS region (or any string for MinIO) |

### Observability

| Variable | Default | Description |
|----------|---------|-------------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://your-super-jaeger.net:4317` | OTLP gRPC collector endpoint |
| `OTEL_SERVICE_NAME` | `YOUR_AWESOME_OTEL_SERVICE` | Service name reported in traces |

## Kubernetes Secrets

When deploying to Kubernetes, these variables are injected via a `api-s3-secrets` secret. The CI pipeline creates this secret automatically from GitLab CI/CD variables:

```
HARBOR_USER, HARBOR_TOKEN  : container registry credentials
S3_ENDPOINT, S3_BUCKET     : S3 / MinIO config
S3_ACCESS_KEY, S3_SECRET_KEY
RUST_LOG
```

See [deployment.md](./deployment.md) for full CI/CD setup.
