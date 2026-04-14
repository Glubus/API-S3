# Deployment

## Docker

Build and run locally:

```bash
docker build -t api-s3 .
docker run -p 3000:3000 --env-file .env api-s3
```

The Dockerfile uses a multi-stage build:
1. `cargo-chef` caches dependency compilation
2. Final stage: `debian:trixie-slim` with non-root `appuser`

## Kubernetes (Kustomize)

The `k8s/` directory contains Kustomize overlays for two environments:

```
k8s/
├── base/               # shared manifests (Deployment, Service, Ingress)
└── overlays/
    ├── production/     # namespace: your-super-namespace
    └── pre-prod/       # namespace: your-super-namespace-preprod
```

### Manual deploy

```bash
# Production
kubectl apply -k k8s/overlays/production

# Pre-prod
kubectl apply -k k8s/overlays/pre-prod
```

### Configuration required

Before deploying, update these files with your own values:

| File | Field | Description |
|------|-------|-------------|
| `.ci/base.yml` | `HARBOR_REGISTRY` | Your container registry |
| `.ci/base.yml` | `KUBE_CONTEXT` | Your kubectl context |
| `.ci/base.yml` | `K8S_NAMESPACE` | Production namespace |
| `.ci/base.yml` | `PREPROD_K8S_NAMESPACE` | Pre-prod namespace |
| `k8s/overlays/production/kustomization.yml` | `namespace` + `newName` | Namespace + registry image path |
| `k8s/overlays/pre-prod/kustomization.yml` | `namespace` + `newName` | Namespace + registry image path |
| `k8s/overlays/production/ingress-patch.yml` | `host` | Production hostname |
| `k8s/overlays/pre-prod/ingress-patch.yml` | `host` | Pre-prod hostname |
| `k8s/base/deployment.yml` | `OTEL_EXPORTER_OTLP_ENDPOINT` | Your Jaeger/Tempo collector |

## GitLab CI/CD

The pipeline runs automatically on `master`, `pre-prod`, `dev`, and MRs.

Stages:

| Stage | Jobs | When |
|-------|------|------|
| build | `build-lib-docker` | all branches + MRs |
| quality | `lint-docker`, `test-docker`, `fmt-docker` | all branches + MRs |
| sonar | `sonar` | `pre-prod` only |
| package | `package-prod`, `package-preprod` | `master` / `pre-prod` |
| deploy | `deploy-cluster`, `deploy-preprod` | `master` / `pre-prod` |

### Required CI/CD variables

Set these in your GitLab project settings under **Settings → CI/CD → Variables**:

| Variable | Description |
|----------|-------------|
| `HARBOR_USER` | Registry username |
| `HARBOR_TOKEN` | Registry password / token |
| `S3_ENDPOINT` | S3/MinIO URL |
| `S3_BUCKET` | Bucket name |
| `S3_ACCESS_KEY` | S3 access key |
| `S3_SECRET_KEY` | S3 secret key |
| `SONAR_HOST_URL` | SonarQube server URL |
| `SONAR_TOKEN` | SonarQube token |
| `CI_BOT_TOKEN` | GitLab token for version-bump job |
