# Migrating from AppImage to erebus Agents

This guide walks you through packaging an existing Python/Node/Go application
as an erebus `.erebus` binary instead of an AppImage.

## Key Differences

| Aspect | AppImage | erebus |
|--------|----------|--------|
| Format | SquashFS + ELF loader | `[stub][payload][metadata][footer]` |
| Entrypoint | AppRun script | `runtime` + `entrypoint` in metadata |
| Signature | GPG external | Ed25519 embedded in footer |
| Delta updates | AppImageUpdate (zsync) | SISR (FastCDC chunks + Merkle manifest) |
| Isolation | None (filesystem access) | Seccomp + Landlock (configurable) |
| AI agents | Not supported | `artifact_type=agent` + model layer |

## Quick Migration

### From AppImage to erebus

```bash
# OLD: AppImage
./myapp.AppImage --appimage-extract
./squashfs-root/AppRun

# NEW: erebus
erebus build ./myapp -o myapp.erebus
./myapp.erebus
```

### AppImage structure → erebus structure

```
# AppImage
AppRun                    →  runtime + entrypoint in metadata
myapp.desktop             →  (embedded in metadata.name/description)
squashfs-root/            →  payload (zstd+tar or squashfs)
  usr/bin/myapp           →  payload/app
  usr/lib/...             →  payload/usr/lib (or embedded runtime via --embed-interpreter)

# erebus
[stub][payload][metadata][footer]
  ↑      ↑         ↑        ↑
  ELF    zstd      JSON     constants
```

## Python AppImage Migration

```bash
# If you have a Python AppImage with embedded interpreter
# erebus can embed the interpreter directly — no AppRun needed

erebus build ./myapp \
  --embed-interpreter python3 \
  --include .venv \
  -o myapp.erebus
```

The `--embed-interpreter python3` flag bundles a Python 3 interpreter into
the binary, eliminating the need for a system Python or AppRun shim.

## AI Agent Migration

If you're packaging an AI agent (model + prompt + runtime):

```bash
erebus build ./my-agent \
  --artifact-type agent \
  --embed-interpreter python3 \
  --embed-model ./model.gguf \
  --include ./tools \
  --include ./prompts \
  -o agent.erebus
```

Key differences from AppImage:
- `--artifact-type agent` enables agent-specific metadata
- `--embed-model` tunes FastCDC chunk size to 16 MiB (vs default 64 KiB)
- `--include` bundles prompt templates and tool scripts as separate files

## Signature Migration

AppImage uses GPG signatures (external `.asc` file):

```bash
# AppImage
gpg --detach-sign myapp.AppImage

# erebus — Ed25519 signature embedded in footer
erebus keygen --output ~/.erebus/keys/
erebus sign myapp.erebus --key ~/.erebus/keys/*.key
erebus verify myapp.erebus
```

## Delta Update Migration

AppImage uses `zsync` for delta updates:

```bash
# AppImage
appimagetool --target-url=http://example.com/myapp.AppImage.zsync

# erebus — SISR (self-indexing stream reconstruction)
erebus swap myapp.erebus model ./new-model.gguf --sisr-delta --key ~/.erebus/keys/*.key
```

SISR advantages:
- No external `.zsync` file needed — manifest is embedded
- Chunk-level SHA-256 verification on every fetch
- Works with any payload type (models, code, config)

## Multi-Agent Packaging

Unlike AppImage (single app per file), erebus supports multiple agents:

```bash
erebus build ./agents \
  --artifact-type agent \
  --entrypoint chat=chat.py \
  --entrypoint code=code.py \
  -o multi-agent.erebus

# Select agent at runtime
./multi-agent.erebus chat
./multi-agent.erebus code
```

## Isolation Comparison

AppImage applications have full filesystem access. erebus adds:

```bash
# Default: namespace + seccomp + landlock (sandbox)
./app.erebus

# Less isolation (for dev):
./app.erebus --isolation none

# Feature flags for debugging:
./app.erebus --no-sisr    # skip delta updates
./app.erebus --no-sign    # skip signature verification (DEV ONLY)
```

## Troubleshooting

| Issue | AppImage fix | erebus fix |
|-------|-------------|------------|
| Missing library | `linuxdeploy --appdir usr/lib` | erebus bundles automatically |
| Wrong interpreter | `AppRun` shim | `--embed-interpreter` |
| Update fails | Re-download full file | `erebus swap` with `--sisr-delta` |
| Can't write to FS | `--appimage-extract` + patch | `--isolation none` or `--no-sisr` |
