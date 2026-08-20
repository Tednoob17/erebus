# IDEAS.md — erebus & Beyond

Collected ideas, research findings, and future directions. Includes both what erebus can do today and what would need separate tools.

---

## What erebus Solves Today (Linux Edge Gateways)

### Core Value Prop
**Single self-extracting ELF** containing: `[stub][runtime][model][prompt][tools][config]`
- No daemon, no container runtime, no registry required
- Runs on any Linux (Yocto, Buildroot, Debian, Ubuntu Core, Alpine)
- ARM64, x86_64, RISC-V (musl static)

### Concrete IoT Use Cases (Linux-based)

| Segment | Devices | Pain Point | erebus Solution |
|---------|---------|------------|-----------------|
| **Industrial edge gateways** | RK3566, i.MX8, Jetson Orin, Pi CM4, Advantech, Moxa | Full OS rebuild (100-500MB) for model update | `erebus swap model` + SISR delta (5-10MB) |
| **Smart cameras / vision AI** | Jetson, Coral, Luxonis OAK, custom ARM64 | Model drift, cellular bandwidth cost | Delta update over 4G/LoRa, atomic rollback |
| **PLC / controllers (Linux)** | Revolution Pi, Moxa UC, Advantech UNO | Safety certification, IEC 62443 | Ed25519 signing, capability sandbox, health-gated rollback |
| **Energy / smart grid** | Substation gateways, solar inverters, EV chargers | Remote, intermittent connectivity, CRA compliance | Offline-first, signed updates, capability-driven (no net if not declared) |
| **Fleet / telematics** | Vehicle gateways, railway wayside boxes | Staged rollout, canary, health-gated rollback | Health gate + fleet manager integration |

### What's Already Implemented
- ✅ Phase 1: Stub reads `layers[]`, `effective_entrypoint()`, `capabilities()`
- ✅ Phase 2: `erebus swap <binary> <layer> <file> --sisr-delta --key`
- ✅ Phase 5b: Capability-driven seccomp (Exec/Network/Syscall) + Landlock (WriteFile)
- ✅ Security: Ed25519 signing, AES-256-GCM encryption, lru soundness fix
- ✅ SISR: Content-defined chunking, Merkle tree, delta updates, health-gated rollback

---

## What erebus CANNOT Do Today (Bare-Metal MCU / TinyML)

### The Gap
erebus stub = Linux-only (`pivot_root`, `libc`, `execvp`, Linux syscalls).
**Does not run on:** Cortex-M, ESP32, Zephyr, FreeRTOS, bare metal.

### Existing Solutions (Don't Reinvent)
| Tool | Approach | Target |
|------|----------|--------|
| **Minerva** (EuroSys'26) | Linker-isolated "capsules" with fixed `predict()` entrypoint; in-place `memcpy` swap, no reboot | ARM Cortex-M (nRF52840), Zephyr-ready |
| **TinyMLDelta** | Binary diff patches (RLE + CRC32), A/B flash slots, crash-safe journal | TFLite Micro, Arduino UNO Q, Zephyr, ESP32 |
| **RIOT-ML** | SUIT manifests (RFC 9019), CoAP registry, uTVM compilation | RIOT OS, Cortex-M, ESP32, RISC-V |
| **Zephyr LLEXT** | Linkable Loadable Extensions — load ELF modules at runtime | Zephyr RTOS, 500+ boards |
| **mallorn** | Tensor-aware diffing (ZipNN), GGUF/TFLite/ONNX/SafeTensors, Ed25519, WASM | MCU-friendly streaming patch apply |

### What Would Be Needed for erebus on MCU (Separate Tool: `erebus-mcu`)

| Component | Effort | Notes |
|-----------|--------|-------|
| **Freestanding stub** | High | No `libc`, no `pivot_root`. Custom linker script for capsule regions. |
| **Capsule runtime** | High | Linker-isolated `.container-code` / `.container-data` sections, fixed `predict()` address. |
| **SUIT manifest (RFC 9019)** | Medium | Standard for MCU OTA. Integrate with `cbor` crate. |
| **A/B flash slot manager** | Medium | Atomic slot flip, crash-safe journal (power-loss recovery). |
| **Flash-aware `AtomicWriter`** | Medium | Port current `AtomicWriter` to flash partitions (not filesystem). |
| **Delta patcher** | Low | Reuse `mallorn` / `TinyMLDelta` logic for tensor-aware diffs. |
| **Zephyr/RTOS integration** | Medium | LLEXT for Zephyr, native for bare metal. |

### Architecture Sketch: `erebus-mcu`
```
[bootloader][slot A: capsule v1][slot B: empty][SUIT manifest][metadata]
```
- Bootloader verifies SUIT manifest → jumps to active slot
- Update: download patch → write to inactive slot → verify CRC32/SHA256 → atomic slot flip
- Capsule = fixed-address `predict()` + weights; app calls `predict()` via function pointer
- No reboot needed for weights-only / operators-only updates

---

## Future Ideas (Could Be Separate Tools)

### 1. `erebus-registry` — CAS Registry Server
- **What**: HTTP/CoAP server for layer push/pull, artifact publish
- **Why**: Share `python3` runtime layer across 100 agents (dedup)
- **Protocol**: S3-compatible or custom (layer hash = content address)
- **Auth**: Token-based, integrate with `erebus keygen`/`trust`

### 2. `erebus-fleet` — Fleet Manager / OTA Orchestrator
- **What**: Central control plane for staged rollout, canary, rollback
- **Features**:
  - Device groups, tags, blueprints (like Golioth)
  - Health telemetry ingestion (inference latency, memory, temp)
  - Auto-rollback on health gate failure
  - Freeze/rollback windows per device group
- **Integration**: `erebus run` health gate → POST to fleet manager

### 3. `erebus-sbom` — SBOM Generator
- **What**: Extract SPDX/CycloneDX from `metadata.layers[]`
- **Why**: CRA compliance, supply chain transparency
- **Input**: `erebus inspect --sbom binary.ere` → SPDX JSON

### 4. `erebus-wasm` — WASM Component Support
- **What**: `ToolLayer` with `command: ["wasmtime", "component.wasm"]`
- **Why**: Polyglot plugins, sandboxed extensions
- **Capability**: `Capability::Wasm` → seccomp allow `mmap`/`mprotect` for JIT

### 5. `erebus-template` — Project Scaffolding
- **What**: `erebus init --template agent|service|plugin`
- **Generates**: `erebus.toml` with correct `ArtifactType`, layers, capabilities
- **Templates**:
  - `agent`: model + prompt + runtime + tools + caps (Network, Exec)
  - `service`: multi-entrypoint, health check, config layer
  - `plugin`: WASM component, no network, read-only rootfs

### 6. `erebus-bench` — Benchmark Suite
- **What**: Standardized benchmarks for cold start, inference latency, update size
- **Targets**: Compare erebus vs Docker vs native vs Ollama on same hardware
- **CI Integration**: Run on PR, track regressions

---

## Research-Backed Priorities (From Literature)

| Priority | Source | Action |
|----------|--------|--------|
| **Delta updates mandatory** | RIOT-ML: 99% network savings for weights-only | ✅ SISR done |
| **A/B partitions / atomic swap** | Minerva, TinyMLDelta, Simms, RIoT | ✅ `AtomicWriter` + rename; MCU needs flash slots |
| **Health-gated rollback** | Simms, RIoT Secure, Golioth, zed.io | ✅ Health gate in stub; needs fleet integration |
| **Capability-driven sandbox** | RIoT Secure, CRA, IEC 62443 | ✅ Phase 5b done |
| **Model fingerprinting** | mallorn: ~10ms version detection | Add to `erebus inspect` |
| **Tensor-aware diffing** | mallorn (ZipNN), TinyMLDelta | For `erebus-mcu` patches |
| **Lifecycle as architecture** | RIoT Secure, CRA 2024 | `erebus-fleet` + `erebus-registry` |

---

## Decision Matrix: When to Build What

| If User Needs... | Build / Use |
|------------------|-------------|
| Linux edge gateway, single binary deploy | **erebus** (today) |
| Bare-metal MCU (Cortex-M, ESP32), TinyML | **Minerva** / **TinyMLDelta** / **Zephyr LLEXT** |
| Fleet orchestration, staged rollout | **erebus-fleet** (future) / Golioth / Balena |
| Layer sharing across artifacts | **erebus-registry** (future) |
| Regulatory SBOM | **erebus-sbom** (future) |
| WASM plugins | **erebus-wasm** (future) |

---

## Open Questions

1. **SUIT vs custom manifest** — SUIT is standard but complex; custom is simpler but non-interop.
2. **Zephyr LLEXT vs custom capsule** — LLEXT is Zephyr-only; capsule is portable but more work.
3. **Registry protocol** — S3-compatible (ecosystem) vs custom (simpler, content-addressed).
4. **Health telemetry schema** — Standardize metrics (latency p50/p99, mem, temp, inference success rate).

---

## Related Projects to Watch

- **Minerva** (github.com/ShishirPatil/minerva) — MCU capsules, EuroSys'26
- **TinyMLDelta** (github.com/felixgalindo/TinyMLDelta) — Binary patches for TFLite Micro
- **RIOT-ML** (github.com/TinyPART/RIOT-ML) — SUIT + uTVM + CoAP
- **mallorn** (github.com/gar-ai/mallorn) — Tensor-aware diffs, GGUF/TFLite/ONNX
- **Zephyr LLEXT** (antmicro.com/blog/2024/12/...) — Runtime-loadable extensions
- **Golioth** (blog.golioth.io) — TF Lite OTA via artifacts
- **ConceptOS** (IEEE IoTJ 2024) — Microkernel component updates with live state transfer

---

*Last updated: 2026-08-20*
*Based on: RIOT-ML (2024), Minerva (EuroSys'26), TinyMLDelta, mallorn, Zephyr LLEXT, Golioth, RIoT Secure, Simms, CRA 2024, IEC 62443*