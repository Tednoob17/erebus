# Rollback After Bad Deploy

## Overview

If a SISR update or new artifact version causes the application to fail health
checks, the erebus stub automatically rolls back to the previous known-good
version. This runbook documents the rollback flow and manual recovery steps.

## Automatic Rollback Flow

```
┌──────────────────┐
│  SISR update      │  Stub fetches remote manifest + delta chunks
│  applied         │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Health gate      │  App starts supervised, polled for `timeout_ms`
│  supervised       │
└────────┬─────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
 healthy   unhealthy/crash
    │         │
    ▼         ▼
 mark_healthy  record_failure
    │         │
    │    ┌────┴────┐
    │    │ attempts│
    │    │ >= max? │
    │    └────┬────┘
    │         │
    │    ┌────▼───┐
    │    │quarantine│
    │    └────┬────┘
    │         ▼
    │    rollback_to_previous
    │         │
    └─────────┴───────► exec_again(canonical_path)
```

## When Automatic Rollback Triggers

1. **Crash on startup** — app exits within `ERE_HEALTH_TIMEOUT_MS`
   (default: 5000ms).
2. **Health check failure** — app exits non-zero OR health endpoint
   returns non-200 within the grace window.
3. **Quarantine** — version has `max_attempts` (default: 3) cumulative
   failures recorded in `HealthStore`.

## Where State Lives

- **Health store**: `~/.cache/erebus/health/` (JSON files per version hash)
- **Backup binary**: `~/.cache/erebus/.backup/<sha256>` (previous version)
- **Lock file**: `~/.cache/erebus/<sha256>.lock` (flock during extraction)

## Manual Recovery

### If `--no-sisr` is set but app still fails

```bash
# Disable SISR entirely
./app.erebus --no-sisr

# Bypass Ed25519 signature verification (dev only)
./app.erebus --no-sign
```

### If the binary is quarantined

```bash
# Clear health store for this version
rm ~/.cache/erebus/health/<sha256>

# Or clear all health records (use with caution)
rm -rf ~/.cache/erebus/health/

# Clear the cache entirely and force re-extraction
rm -rf ~/.cache/erebus/
```

### If SISR update is stuck

```bash
# Remove ERE_SISR_MANIFEST to skip delta update
unset ERE_SISR_MANIFEST

# Or run with --no-sisr to skip SISR entirely
./app.erebus --no-sisr
```

### If pivot_root fails

```
[erebus] warning: pivot_root failed (Permission denied) — falling back to bind-mount-only
```

This means the process lacks `CAP_SYS_ADMIN` or is running in a restricted
container. The stub falls back to running without namespace isolation. To fix:

```bash
# Check capabilities
capsh --print

# If running in Docker, add sys_admin:
docker run --privileged  # or --cap-add SYS_ADMIN
```

### If seccomp returns ENOSYS

```
[erebus] warning: seccomp unavailable on this kernel — continuing without syscall filter
```

The kernel doesn't support seccomp. All security enforcement is skipped.

## Debugging Steps

1. **Enable verbose logging**: `ERE_VERBOSE=1 ./app.erebus`
2. **Check health store**: `ls -la ~/.cache/erebus/health/`
3. **Inspect metadata**: `erebus inspect app.erebus`
4. **Validate integrity**: `erebus selftest app.erebus`
5. **Check SISR manifest**: `ERE_SISR_MANIFEST=/path/to/manifest ./app.erebus`

## Configuration Flags

| Flag | Env Var | Effect |
|------|---------|--------|
| `--no-sisr` | `ERE_NO_SISR=1` | Skip SISR delta update, force classic extraction |
| `--no-sign` | `ERE_NO_SIGN=1` | Skip Ed25519 signature verification (dev only) |
| `--erebus-update=URL` | `ERE_UPDATE_URL` | Trigger SISR update from specified URL |
| `--erebus-version` | — | Print version info and exit |
| `--erebus-agent NAME` | — | Select multi-agent entrypoint (Phase 6) |

## Health Check Policy Defaults

| Parameter | Default | Env Override |
|-----------|---------|--------------|
| `timeout_ms` | 5000 | `ERE_HEALTH_TIMEOUT_MS` |
| `max_attempts` | 3 | `ERE_HEALTH_MAX_ATTEMPTS` |
| `health_endpoint` | `/health` | (in metadata) |
| `health_port` | (from metadata) | `ERE_HEALTH_PORT` |
