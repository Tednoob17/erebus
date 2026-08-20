# erebus — ROADMAP

## Vision

erebus is a **universal self-extracting artifact format** — a single ELF binary
containing `[stub][runtime][model][prompt][tools][config]` with no daemon, no
container runtime, no host dependencies.

### Supported artifact types
| Type | Status |
|------|--------|
| Application | ✅ Production-ready |
| AI Agent | ✅ (model + prompt + runtime + tools) |
| Service / Multiservice | ✅ (`services[]` in metadata) |
| Plugin | ✅ (`ArtifactType::Plugin`) |
| Multi-agent | ✅ (one binary, multiple entrypoints via `--erebus-agent`) |

### IoT targeting (Linux edge gateways)
erebus solves real problems for edge AI deployment:

| Segment | Devices | Problem | erebus Solution |
|---------|---------|---------|-----------------|
| **Industrial edge gateways** | RK3566, i.MX8, Jetson Orin, Pi CM4 | 100-500MB OS rebuild for model update | `erebus swap model` + SISR delta (5-10MB) |
| **Smart cameras / vision AI** | Jetson, Coral, Luxonis OAK | Model drift, cellular bandwidth cost | Delta update over 4G/LoRa, atomic rollback |
| **PLC / controllers (Linux)** | Revolution Pi, Moxa UC, Advantech UNO | IEC 62443 compliance | Ed25519 signing, capability sandbox, health-gated rollback |
| **Energy / smart grid** | Substation gateways, EV chargers | Remote, intermittent connectivity, CRA | Offline-first, signed updates, capability-driven (no net unless declared) |
| **Fleet / telematics** | Vehicle gateways, railway boxes | Staged rollout, canary, rollback | Health gate + fleet manager integration |

---

## Completed Phases

### Phase 1: Layers in stub ✅
Stub consumes `ArtifactMetadata.layers` — reads `layers[]`, `entrypoint_layer`,
resolves entrypoint via `effective_entrypoint()`/`effective_runtime()`/`capabilities()`.
The `Entrypoint` trait from core is **not** used by the stub (deliberate — stub
contains `unsafe` `execvp` that core cannot).

### Phase 2: Hot-swap (SISR) ✅
`erebus swap <binary> <layer> <file> [--sisr-delta --key]` — replaces a single
layer without full rebuild. Delta computed at runtime via chunk reuse.

### Phase 3: Lazy loading ✅
`LazyFs` FUSE mount (`fuser` + `backhand`) — serves squashfs/zstd-tar layers on-demand
without full extraction. Activated via `ERE_LAZY_LOAD=1`, falls back to full extract.

### Phase 4: Registry CAS ✅
`Registry` trait + `MemoryRegistry` + `DiskRegistry` in `erebus-core/src/registry.rs`.
CLI: `erebus registry push-layer|pull-layer|list-layers|publish-artifact|inspect-artifact|list-artifacts`.
HTTP transport via `HttpRegistryClient` (reqwest, blocking, Bearer auth).

### Phase 5a: Security baseline ✅
Seccomp BPF (18 syscalls blocked), Landlock LSM, Ed25519 signing, AES-256-GCM
encryption, health-gated rollback, zeroize, Clé perms 0o600. 35 findings reviewed
(17 true positives → all fixed). lru soundness fix (ratatui 0.29→0.30.2).

### Phase 5b: Capability-driven security ✅
`SeccompConfig::from_capabilities()` → dynamic BPF filter.
`LandlockConfig::from_capabilities()` → R/W rootfs if `WriteFile`, read-only otherwise.
Capabilities: `ReadFile`, `WriteFile`, `Network`, `Exec`, `Syscall`, `Env`.

### Phase 6: Templates + multi-agent ✅
`ArtifactType` enum (`Application`/`Agent`/`Service`/`Plugin`). Multi-agent entrypoints
map + `--erebus-agent` selector. CLI flags: `--artifact-type`, `--entrypoint NAME=args`.
Examples in `examples/`, migration guide in `docs/migration/`.

### Phase 7: Operational hardening ⏳ (partial)
- ✅ Observability: stderr logging `[erebus] ...` on all phases
- ✅ Health gate: quarantine + atomic rollback (`HealthStore`)
- ✅ Atomic swap: `AtomicWriter` with rollback snapshot
- ✅ Signal handler: async-signal-safe (`static mut + AtomicUsize`)
- ✅ HTTP 64 MiB limit (configurable via `ERE_HTTP_MAX_RESPONSE`)
- ✅ Zeroize all crypto keys, privkey perms 0o600
- ✅ `erebus selftest` CLI command
- ✅ Feature flags: `--no-sisr` / `--no-sign` (via `ERE_NO_SISR`/`ERE_NO_SIGN`)
- ✅ Error rescue: pivot_root EPERM/EINVAL → extract-only fallback, seccomp ENOSYS → warning
- ✅ Rollback runbook: `docs/runbooks/rollback-after-bad-deploy.md`
- ❌ **Missing**: Prometheus metrics / `--metrics :9090`, `SECURITY-AUDIT.md`,
  ASCII diagrams in source code, chaos testing CLI

---

## What erebus CANNOT Do (MCU / Bare Metal)

erebus stub is Linux-only (`pivot_root`, `libc`, `execvp`). Does not run on Cortex-M,
ESP32, Zephyr, FreeRTOS, bare metal.

### Existing solutions (do not reinvent)
| Tool | Approach | Target |
|------|----------|--------|
| **Minerva** (EuroSys '26) | Linker-isolated capsules, in-place `memcpy` swap | ARM Cortex-M, Zephyr |
| **TinyMLDelta** | Binary diff patches (RLE+CRC32), A/B flash slots | TFLite Micro, Zephyr, ESP32 |
| **RIOT-ML** | SUIT manifests (RFC 9019), CoAP registry, uTVM | ARM Cortex-M, ESP32, RISC-V |
| **Zephyr LLEXT** | Linkable Loadable Extensions | Zephyr RTOS, 500+ boards |
| **mallorn** | Tensor-aware diffing (ZipNN), GGUF/TFLite/ONNX | MCU-friendly streaming patches |

---

## Future Roadmap

### Short term (Phase 7 completion)
1. **Prometheus metrics** — `--metrics :9090` exposing cold start, cache hits,
   update size, health gate status
2. **`SECURITY-AUDIT.md`** — full audit report with findings + mitigations
3. **ASCII diagrams in source** — embed in `main.rs`, `exec.rs`, `seccomp.rs`
4. **Chaos testing CLI** — expose fault injection as `erebus test chaos`

### Future tools (separate crates)
| Tool | Purpose |
|------|---------|
| **erebus-registry** | HTTP/CoAP server for layer push/pull, artifact publish |
| **erebus-fleet** | Fleet manager: staged rollout, canary, health telemetry |
| **erebus-sbom** | Generate SPDX/CycloneDX from `metadata.layers[]` |
| **erebus-wasm** | WASM component support (`Capability::Wasm` → seccomp allow `mmap`/`mprotect`) |
| **erebus-template** | Project scaffolding (`erebus init --template agent\|service\|plugin`) |
| **erebus-bench** | Standardized benchmarks (cold start, inference latency, update size) |
| **erebus-mcu** | Bare-metal MCU support (capsules, SUIT manifests, A/B flash slots) |

### Research-backed priorities
| Priority | Source | Action |
|----------|--------|--------|
| Delta updates | RIOT-ML: 99% network savings for weights-only | ✅ SISR done |
| A/B partitions / atomic swap | Minerva, TinyMLDelta, Simms, RIoT | ✅ `AtomicWriter` + rename; MCU needs flash slots |
| Health-gated rollback | Simms, RIoT Secure, Golioth, zed.io | ✅ Health gate in stub; fleet integration needed |
| Capability-driven sandbox | RIoT Secure, CRA, IEC 62443 | ✅ Phase 5b done |
| Model fingerprinting | mallorn: ~10ms version detection | Add to `erebus inspect` |
| Tensor-aware diffing | mallorn (ZipNN), TinyMLDelta | For `erebus-mcu` patches |

---

## Decision Matrix

| If User Needs... | Use |
|------------------|-----|
| Linux edge gateway, single binary deploy | **erebus** (today) |
| Bare-metal MCU (Cortex-M, ESP32), TinyML | **Minerva** / **TinyMLDelta** / **Zephyr LLEXT** |
| Fleet orchestration, staged rollout | **erebus-fleet** (future) / Golioth / Balena |
| Layer sharing across artifacts | **erebus-registry** (future) / S3-compatible |
| Regulatory SBOM | **erebus-sbom** (future) |
| WASM plugins | **erebus-wasm** (future) |

---

## Single Points of Failure

| SPOF | Risk | Mitigation | Status |
|------|------|------------|--------|
| SISR manifest server | Network outage → updates fail | HTTP retry; fallback to classic extraction if manifest unavailable | ⚠️ Partial |
| Cache corruption (`~/.cache/erebus/<hash>/`) | Corrupted cache → wrong behavior | `cache_root_trustworthy()` checks ownership + SHA-256 verify | ✅ Mitigated |
| CI musl cross-compilation | musl toolchain unavailable → no ARM builds | CI builds on x86_64 musl only; no multi-arch yet | ⚠️ Needs improvement |
| Ed25519 signing key compromise | Attacker forges binaries | `ed25519-dalek >= 2.1` enforces key clamping (CVE-2023-48022); key perms 0o600 at build | ✅ Mitigated |

---

## Technical Constraints

- **vfat**: repo lives on vfat (no exec bit). Cargo target dir is `/tmp/erebus-stub-target`.
- **musl**: stub builds with `--target x86_64-unknown-linux-musl` for static ELF.
- **CI**: clippy runs per-crate, not workspace-wide.
- **Edition**: Rust 2021, stable only (no nightly/beta).
- **Safety**: zero `unsafe` in `erebus-core` and `erebus-cli`. All `unsafe` in `erebus-stub`.

---

*Last updated: 2026-08-20*
