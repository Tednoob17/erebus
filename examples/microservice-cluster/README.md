# Microservice Cluster Example

A single erebus binary containing 3 Python microservices (auth, API, worker)
with health checks and auto-restart. Uses `--artifact-type service` and the
`services` metadata field.

## Structure

```
microservice-cluster/
├── config.json           # Service definitions (entrypoint, port, health check)
├── services/
│   ├── auth/app.py       # Auth service (port 8001, JWT tokens)
│   ├── api/app.py        # Main API (port 8000, /health + /api/message)
│   └── worker/app.py     # Background worker (periodic batch processing)
└── requirements.txt
```

## Build

```bash
erebus build ./microservice-cluster --artifact-type service \
  --embed-interpreter python3 \
  -o microservice-cluster.erebus
```

## Usage

```bash
./microservice-cluster.erebus
# Starts all 3 services with health gates

# Individual service ports:
#   auth:   http://127.0.0.1:8001
#   api:    http://127.0.0.1:8000
#   worker: (background, no HTTP)
```

## How it works

The stub's `supervise_services()` (main.rs:410) reads `meta.services` from
metadata and spawns each service as a supervised subprocess. Each service
runs with the same rootfs and isolation level.
