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

### Phase 6: Templates + multi-agent ✅
`ArtifactType` enum (`Application`/`Agent`/`Service`/`Plugin`). Multi-agent entrypoints
map + `--erebus-agent` selector. CLI flags: `--artifact-type`, `--entrypoint NAME=args`.
Examples in `examples/`, migration guide in `docs/migration/`.

### Phase 7b: Desktop packaging ✅
`--package-format flatpak|snap` flag generates distribution-ready packaging files.
`PackageFormat` enum in `erebus-core/src/flatpak.rs`. Flatpak: `flatpak-manifest.json` + `.desktop`.
Snap: `snapcraft.yaml` with apps/services/confinement. Runtime→Flatpak runtime mapping.

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

### Phase 7b: Desktop packaging ✅
`--package-format flatpak|snap` flag generates distribution-ready packaging files:
- **`PackageFormat` enum** (`Erebus`/`Flatpak`/`Snap`) in `erebus-core/src/flatpak.rs`
- **Flatpak**: generates `flatpak-manifest.json` + `.desktop` file with runtime mapping
  (python/node/ruby/etc → `org.freedesktop.Platform`)
- **Snap**: generates `snapcraft.yaml` with app/service definitions, confinement, plugs
- Runtime → Flatpak runtime mapping: `runtime_mapping()` function
- `erebus build ./app --package-format flatpak -o app.flatpak` → emits manifest + desktop

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

### Phase 8: Universal Build Compatibility (100% GitHub Trending Coverage)

This phase aims to make erebus build **any repo on GitHub trending** across all languages.
Priority order based on GitHub trending frequency + impact.

#### 8.1 Missing Runtime Support (New Languages)

| Priority | Runtime | Build System | Key Challenges | Effort |
|----------|---------|--------------|----------------|--------|
| **P0** | **Rust** | Cargo | Workspaces, proc-macros, build.rs, cross-compile, proc-macro crates | High |
| **P0** | **Go** | Go Modules | Private modules, CGO, cross-compile, go.work, replace directives | High |
| **P1** | **Java** | Maven/Gradle | Wrapper scripts (mvnw/gradlew), multi-module, JDK version detection | Medium |
| **P1** | **.NET** | MSBuild/dotnet CLI | csproj/sln, NuGet.config, global.json, dotnet publish | Medium |
| **P2** | **Swift** | SwiftPM | Not yet supported | Medium |
| **P2** | **Kotlin** | Gradle/Maven | Same as Java + Kotlin DSL | Medium |
| **P3** | **Scala** | sbt | Not supported | Low |
| **P3** | **C/C++** | CMake/Make/Meson | Build system diversity | Low |
| **P3** | **Zig** | Zig build | Not supported | Low |
| **P3** | **Nim** | Nimble | Not supported | Low |
| **P3** | **Haskell** | Cabal/Stack | Not supported | Low |

**Implementation approach per language:**
- Detect via marker files (Cargo.toml, go.mod, pom.xml, build.gradle, csproj, Package.swift, etc.)
- Auto-detect package manager (Cargo, go modules, Maven/Gradle, dotnet, SwiftPM)
- Implement build step: `cargo build --release`, `go build`, `mvn package`, `dotnet publish`, etc.
- Cross-compile: `--target` for Rust/Go, multi-arch JDK for Java, `dotnet publish -r`
- Embed runtime: static binary for Go/Rust, JDK for Java, .NET runtime for .NET

#### 8.2 Edge Cases for Existing Runtimes (Hardened)

##### Node.js / TypeScript
| Edge Case | Status | Implementation |
|-----------|--------|----------------|
| **pnpm workspaces** (monorepo) | ❌ | Detect `pnpm-workspace.yaml` + `workspace:*` protocol; run `pnpm install -r --offline` |
| **Yarn Berry (v2/v3)** | ❌ | Detect `yarnrc.yml` + `.yarn/` + `packageManager` field; use Corepack |
| **Turborepo** | ❌ | Detect `turbo.json`; run `turbo run build --filter=...` |
| **Nx workspace** | ❌ | Detect `nx.json` + `workspace.json`; run `nx run-many --target=build` |
| **TypeScript project references** | ❌ | Detect `references` in `tsconfig.json`; build in dependency order |
| **Yarn workspaces v1** | ✅ Partial | Already works via `yarn.lock` |
| **npm workspaces** | ⚠️ | Detect `"workspaces"` in package.json |
| **Private registries** (npm/pip/cargo/go/composer) | ❌ | Parse `.npmrc`, `pip.conf`, `.cargo/config.toml`, `go.env`, `auth.json` |
| **Native addons** (node-gyp, Cargo, go) | ⚠️ | Detect `binding.gyp`, `Cargo.toml` in deps; compile with target toolchain |
| **TypeScript project references** | ❌ | Build in topological order |
| **Bun** (runtime + pkg mgr) | ⚠️ | Detect `bun.lockb`; use `bun install` + `bun` runtime |
| **ESM only** (`"type": "module"`) | ⚠️ | Detect `"type": "module"` in package.json |
| **Private registries auth** | ❌ | Parse `.npmrc`/`auth.json`/`pip.conf`/`.cargo/config.toml`/`go.env`/`auth.json` |

##### Python
| Edge Case | Status | Implementation |
|-----------|--------|----------------|
| **uv workspaces** (uv 0.5+) | ❌ | Detect `uv.lock` + `[tool.uv.workspace]`; use `uv sync` |
| **Poetry private repos** | ⚠️ | Parse `[tool.poetry.source]` in `pyproject.toml` |
| **Conda environments** | ❌ | Detect `environment.yml`; use `conda env export` |
| **pip private indexes** | ❌ | Parse `pip.conf` / `--index-url` / `--extra-index-url` / `--find-links` |
| **uv + cross-platform wheels** | ❌ | `uv pip compile --platform=linux_x86_64` |
| **Poetry dynamic versioning** | ❌ | Detect `poetry-dynamic-versioning` plugin |
| **Multiple requirements files** | ❌ | Support `requirements-*.txt` patterns |
| **Optional dependencies** | ⚠️ | Support `[project.optional-dependencies]` extras |

##### Go
| Edge Case | Status | Implementation |
|-----------|--------|----------------|
| **Go modules** (`go.mod`) | ⚠️ | Auto-detect; run `go build` / `go build -trimpath` |
| **Private modules** | ❌ | Parse `GOPRIVATE`, `GOPROXY`, `.netrc`, `go env` |
| **go.work workspaces** | ❌ | Detect `go.work`; build all modules |
| **`replace` directives** | ❌ | Resolve local replacements |
| **CGO** | ❌ | Require C compiler + cross-compile toolchain |
| **Cross-compilation** | ❌ | `GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build` |
| **Private registries** | ❌ | `GOPRIVATE`, `.netrc`, `GOPROXY` |

##### Java / JVM
| Edge Case | Status | Implementation |
|-----------|--------|----------------|
| **Maven wrapper** (`mvnw`) | ⚠️ | Detect `.mvn/wrapper/`; run `./mvnw package` |
| **Gradle wrapper** (`gradlew`) | ⚠️ | Detect `gradlew` + `gradle/wrapper/`; run `./gradlew build` |
| **Multi-module** | ⚠️ | Build all modules; handle parent POM |
| **JDK version** | ❌ | Detect `.java-version`, `sdkman`, `jenv` |
| **Private Maven repos** | ❌ | Parse `settings.xml` + `servers` + passwords |

##### .NET
| Edge Case | Status | Implementation |
|-----------|--------|----------------|
| **dotnet publish** | ⚠️ | Run `dotnet publish -c Release -r linux-x64 --self-contained` |
| **NuGet.config** | ❌ | Parse `NuGet.config` for private feeds |
| **global.json** (SDK version) | ❌ | Detect and use correct SDK |
| **NuGet private feeds** | ❌ | Parse `packageSources` + credentials |

#### 8.3 Cross-Cutting Missing Features

| Feature | Priority | Implementation |
|---------|----------|----------------|
| **Private registry auth** (all languages) | P0 | Unified auth config: `erebus.auth.toml` or env vars; inject into build |
| **Cross-compilation** (all languages) | P0 | `--target` flag per language; download target toolchains |
| **Monorepo build orchestration** | P0 | Detect Turborepo/Nx/Cargo/Go/Poetry/uv workspaces; build in dep order |
| **Pre/post build hooks** | P1 | npm: `preinstall`/`prepare`/`postinstall`; Cargo: `build.rs`; Gradle: tasks |
| **Private registry auth** | P0 | Unified auth config file + env var injection |
| **SBOM generation** | P1 | `erebus inspect --sbom` → SPDX/CycloneDX from `layers[]` |
| **Build reproducibility** | P1 | `--reproducible` flag; deterministic timestamps, sorted entries |
| **Build cache sharing** | P1 | Layer caching across builds (Docker-style layer cache) |
| **Build scripts execution** | P0 | Run `npm run build`, `cargo build`, `go build`, `mvn package`, `dotnet publish` |
| **Monorepo workspace support** | P0 | Turborepo/Nx/Cargo/Go/Poetry/uv/pnpm/yarn/npm workspaces |

#### 8.4 Hardcore/Unpredictable Edge Cases (From GitHub Trending)

| Edge Case | Why It Breaks | Mitigation |
|-----------|---------------|------------|
| **Git submodules** (private) | Submodule fetch fails without auth | Detect `.gitmodules`; fetch with credentials |
| **Generated code at build** (protobuf, GraphQL, OpenAPI) | Code gen runs at build time | Detect `buf`, `protoc`, `graphql-codegen`; run before build |
| **WASM + native deps** (wasm-pack + native) | Mixed toolchains | Detect `wasm-pack.toml` + `Cargo.toml` |
| **Next.js 14+ middleware/edge runtime** | Edge runtime different from Node | Detect `middleware.ts` + `runtime = 'edge'` |
| **Next.js App Router + Server Actions** | Server actions need special handling | Detect `app/` + `'use server'` |
| **Astro / SvelteKit / Nuxt 3 / Remix** | Framework-specific build output | Detect config files; run framework build |
| **Electron + native modules** | `node-gyp` + Electron headers | Detect `electron-builder`; use `electron-rebuild` |
| **Tauri** (Rust + Web) | Rust + Web build | Detect `tauri.conf.json`; run `cargo tauri build` |
| **Flutter** (Dart) | Not supported | Detect `pubspec.yaml` |
| **React Native** (Metro + native) | Native iOS/Android build | ❌ Out of scope (mobile) |
| **Private Git deps** (`git+ssh://`, `git+https://`) | Auth required | Parse `package.json`/`Cargo.toml`/`go.mod` for git deps |
| **Huge repos** (>10GB) | Memory/disk pressure | Streaming copy; exclude `.git`; shallow clone |
| **Symlink hell** (node_modules, .pnpm-store) | Copy loops, broken links | `--no-dereference` or skip node_modules |
| **Case-sensitive FS on macOS/Windows** | Path collisions | Normalize paths; detect collisions |
| **Hardcoded paths** (`/usr/bin/python`) | Breaks in sandbox | Rewrite shebangs; use `env` |
| **Runtime version pinning** (`.nvmrc`, `.python-version`, `.tool-versions`) | Version mismatch | Detect and install exact version |
| **Lockfile sync drift** | Lockfile != manifest | Run `npm ci` / `pip install --require-hashes` / `cargo fetch` |
| **GitHub Actions / CI-only deps** | Missing in local build | Detect CI-only deps; warn or skip |
| **Secret scanning** (leaked keys in repo) | Security risk | Scan for secrets before build; warn |

#### 8.5 Implementation Priority Order

| Phase | Deliverable | Target |
|-------|-------------|--------|
| **8.1** | Rust (Cargo) + Go support | 2 weeks |
| **8.2** | Java/Gradle/Maven + .NET | 2 weeks |
| **8.3** | Cross-compile (Rust/Go) + Private registries | 2 weeks |
| **8.4** | Monorepo orchestration (Turborepo/Nx/Cargo/Go) | 2 weeks |
| **8.5** | Java/Gradle wrapper + .NET publish | 1 week |
| **8.6** | Swift/Kotlin/Scala (lower priority) | 2 weeks |
| **8.7** | Hardcore edge cases (submodules, code gen, private deps) | 2 weeks |
| **8.8** | Integration testing against 50+ trending repos | 1 week |

---

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
| **100% trending coverage** | GitHub trending analysis | Phase 8 implementation |

---

## Decision Matrix

| If User Needs... | Use |
|------------------|-----|
| Linux edge gateway, single binary deploy | **erebus** (today) |
| Desktop Linux app distribution | `erebus build --package-format flatpak` → Flatpak manifest |
| Desktop Linux/Store app distribution | `erebus build --package-format snap` → snapcraft.yaml |
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
