# Edge Case Testing Plan — Erebus Build Compatibility

Goal: Verify erebus can build trending repos across all supported languages by testing against real-world edge cases from GitHub trending.

---

## Currently Supported Runtimes

| Runtime | Detection | Package Managers | Embed Interpreter | Cross-Compile |
|---------|-----------|------------------|-------------------|---------------|
| Python | ✅ | uv, Poetry, Pipenv, Pip | ✅ python3 | ❌ |
| Node.js | ✅ | npm, pnpm, yarn, bun | ✅ node | ✅ (target platform/arch) |
| Deno | ✅ | (built-in) | ✅ deno | ❌ |
| PHP | ✅ | Composer | ✅ php | ❌ |
| Ruby | ✅ | Bundler | ✅ ruby | ❌ |
| Java | ✅ | Maven, Gradle | ❌ | ❌ |
| .NET | ✅ | (built-in) | ❌ | ❌ |
| Go | ✅ | (built-in) | ❌ | ❌ |
| Perl | ✅ | CPAN | ❌ | ❌ |
| Hugo | ✅ | (built-in) | ✅ hugo | ❌ |
| Wasm | ✅ | (wasmtime) | ✅ wasmtime | ❌ |
| Binary | ✅ | (none) | ❌ | ❌ |
| Electron | ✅ | npm/pnpm/yarn/bun | ✅ electron | ❌ |

---

## Missing Runtimes (Not Supported)

| Runtime | Build System | Notes |
|---------|--------------|-------|
| **Rust** | Cargo | Workspaces, proc-macros, cross-compile |
| **Go** | Modules | Private modules, CGO, `go.work`, `replace` |
| **Java** | Maven/Gradle | Wrapper scripts, multi-module, JDK versions |
| **Swift** | SwiftPM | Not supported |
| **Kotlin** | Gradle/Maven | Not supported |
| **Scala** | sbt | Not supported |
| **C/C++** | CMake/Make/Meson | Not supported |
| **Zig** | Zig build | Not supported |
| **Nim** | Nimble | Not supported |
| **Haskell** | Cabal/Stack | Not supported |

---

## Edge Cases by Language (Likely Trending Repos)

### Node.js / TypeScript

| Edge Case | Status | Test Repo Pattern |
|-----------|--------|-------------------|
| **pnpm workspaces** (monorepo) | ⚠️ Partial | `pnpm-workspace.yaml` + `package.json` workspaces |
| **Yarn workspaces v2/v3 (Berry)** | ⚠️ | `yarn.lock` + `yarnrc.yml` + `.yarn/` |
| **Turborepo** | ⚠️ | `turbo.json` + `package.json` workspaces |
| **Nx workspace** | ⚠️ | `nx.json` + `workspace.json` |
| **TypeScript + native addons** (`.node` files) | ✅ (N-API) | `binding.gyp` / `Cargo.toml` for native |
| **Next.js 14+ App Router** | ⚠️ | `next.config.js` + `app/` directory |
| **Vite + React/Vue/Svelte** | ⚠️ | `vite.config.ts` + `index.html` |
| **Astro** | ⚠️ | `astro.config.mjs` + `.astro` files |
| **Remix** | ⚠️ | `remix.config.js` + `app/` |
| **Nuxt 3** | ⚠️ | `nuxt.config.ts` + `app/` |
| **pnpm + workspace protocol** (`workspace:*`) | ⚠️ | `pnpm-workspace.yaml` + `workspace:*` deps |
| **npm/yarn/pnpm + private registries** | ❌ | `.npmrc` with `registry=https://...` + `_authToken` |
| **npm workspaces** (built-in) | ⚠️ | `package.json` `"workspaces": ["packages/*"]` |
| **Native addons with `node-gyp`** | ⚠️ | `binding.gyp` + `node-gyp` + Python + build tools |
| **ESM only** (`"type": "module"`) | ⚠️ | `"type": "module"` in `package.json` |
| **TypeScript project references** | ❌ | `"references": [{ "path": "./packages/*" }]` |
| **Bun** (as runtime + package manager) | ⚠️ | `bun.lockb` + `bun` runtime |

**Test Strategy**: Clone 5-10 trending TS/JS repos from GitHub trending and attempt `erebus build`.

---

### Python

| Edge Case | Status | Test Repo Pattern |
|-----------|--------|-------------------|
| **uv workspaces** (uv 0.5+) | ❌ | `uv.lock` + `[tool.uv.sources]` + `[tool.uv.workspace]` |
| **Poetry** with private repos | ⚠️ | `poetry.lock` + `[tool.poetry.source]` |
| **Pipenv** + Pipfile.lock | ✅ | `Pipfile` + `Pipfile.lock` |
| **pip + requirements.txt** (private index) | ❌ | `requirements.txt` + `--index-url` + `--extra-index-url` |
| **Pyproject.toml** (PEP 621) + optional deps | ⚠️ | `[project.optional-dependencies]` |
| **uv + pip compile** | ❌ | `uv pip compile pyproject.toml -o requirements.txt` |
| **Django** (manage.py, settings) | ✅ | `manage.py` + `settings.py` |
| **FastAPI** + Uvicorn/Gunicorn | ⚠️ | `main.py` + `uvicorn` |
| **Streamlit** | ⚠️ | `streamlit run app.py` |
| **Gradio** | ⚠️ | `gradio` + `app.py` |
| **uv + cross-platform wheels** | ❌ | `uv pip compile --platform=linux_x86_64` |
| **Pip + `--no-binary`** | ✅ | `--no-binary :all:` |
| **Poetry + dynamic versioning** | ❌ | `version = "0.1.0"` + `poetry-dynamic-versioning` |
| **Monorepo with `pip` + `requirements-*.txt`** | ❌ | Multiple `requirements-*.txt` files |
| **Conda environment** | ❌ | `environment.yml` |

---

### Rust

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **Cargo workspaces** | ❌ | `Cargo.toml` `[workspace]` + multiple crates |
| **Proc-macros** | ❌ | `proc-macro` crate type |
| **Build scripts** (`build.rs`) | ❌ | `build.rs` + `cc` crate / `pkg-config` |
| **Proc-macro + build script deps** | ❌ | `build-dependencies` |
| **Cross-compilation** | ❌ | `--target aarch64-unknown-linux-gnu` |
| **`cargo install` / binary crates** | ❌ | Binary-only crates |
| **Proc-macro + WASM** | ❌ | `wasm-bindgen` + proc-macros |

---

### Go

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **Go modules** (`go.mod`) | ⚠️ | Auto-detected but no build step |
| **Private modules** | ❌ | `GOPRIVATE`, `GOPROXY`, `.netrc` |
| **`go.work` workspaces** | ❌ | `go.work` + `use ./module` |
| **`replace` directives** | ❌ | `replace github.com/x => ./local` |
| **CGO** | ❌ | Requires C compiler + cross-compile toolchain |
| **Cross-compilation** | ❌ | `GOOS=linux GOARCH=arm64 go build` |
| **Embed** (`//go:embed`) | ⚠️ | Static files embedded in binary |
| **Private registries** | ❌ | `GOPRIVATE`, `.netrc` |

---

### Java / JVM

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **Maven** (`pom.xml`) | ⚠️ | Auto-detected, no build step |
| **Gradle** (`build.gradle`) | ⚠️ | Auto-detected, no build step |
| **Maven wrapper** (`mvnw`) | ⚠️ | `mvnw` + `.mvn/wrapper/` |
| **Gradle wrapper** (`gradlew`) | ⚠️ | `gradlew` + `gradle/wrapper/` |
| **Multi-module** | ⚠️ | Parent POM + modules |
| **Wrapper scripts** | ❌ | Need to execute `./mvnw` / `./gradlew` |
| **JDK version** | ❌ | `sdkman`, `jenv`, `.java-version` |

---

### PHP

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **Composer** + private repos | ⚠️ | `repositories` in `composer.json` |
| **Platform requirements** (PHP version) | ✅ | Checked in `check_php_platform_reqs` |
| **Extensions** (`ext-*`) | ⚠️ | Checked but not installed |
| **Laravel** (artisan, Octane) | ✅ | `artisan` + `rr.yaml` for RoadRunner |
| **Symfony** | ⚠️ | `composer.json` + `bin/console` |
| **PHP version constraints** | ✅ | `^8.2`, `>=8.1`, etc. |
| **PHP extensions** | ⚠️ | Not bundled, checked only |

---

### Ruby

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **Bundler** + Gemfile.lock | ✅ | Auto-detected |
| **Private gem sources** | ❌ | `source "https://user:pass@gems.example.com"` |
| **Ruby version** (`.ruby-version`) | ❌ | Not detected |
| **Rails** | ⚠️ | `bin/rails` + `config/application.rb` |
| **Jekyll** | ✅ | `_config.yml` + `Gemfile` |

---

### .NET

| Edge Case | Status | Notes |
|-----------|--------|-------|
| **`csproj` / `sln`** | ⚠️ | Auto-detected |
| **`dotnet restore` / `dotnet publish`** | ❌ | No build step |
| **`NuGet.config`** | ❌ | Private feeds |
| **`global.json`** (SDK version) | ❌ | Not detected |

---

## Missing Features for Trending Repos

### 1. **Pre/Post Build Hooks**
- npm: `preinstall`, `prepare`, `postinstall`, `postpack`
- Python: `pyproject.toml` `[tool.setuptools]` `cmdclass`, `pip` hooks
- Cargo: `build.rs`, `build-dependencies`
- Gradle/Maven: Custom tasks/plugins

### 2. **Private Registries / Auth**
- npm: `.npmrc` with `_authToken`, `registry`
- Cargo: `.cargo/config.toml` `[registries]` + `CARGO_REGISTRIES_*`
- Go: `GOPRIVATE`, `.netrc`, `GOPROXY`
- Python: `pip.conf`, `--index-url`, `--extra-index-url`, keyring
- Composer: `composer.json` `repositories` + `auth.json`

### 3. **Cross-Compilation**
| Runtime | Current | Needed |
|---------|---------|--------|
| Node | ✅ platform/arch + ignore-scripts | ✅ Good |
| Go | ❌ | `GOOS`/`GOARCH`/`CGO_ENABLED` |
| Rust | ❌ | `--target` + cross toolchain |
| Python | ❌ | `pip --platform` + manylinux wheels |
| PHP | ❌ | Static PHP + extensions |
| Ruby | ❌ | Static Ruby + gems |
| Java | ❌ | Multi-arch JDK |
| Rust | ❌ | `cross` or native toolchain |

### 4. **Monorepo / Workspace Support**
| Tool | Status |
|------|--------|
| pnpm workspaces | ⚠️ Partial |
| Yarn workspaces v1 | ✅ (lock file) |
| Yarn workspaces v2+ (Berry) | ❌ |
| npm workspaces | ⚠️ |
| Turborepo | ❌ |
| Nx | ❌ |
| Cargo workspaces | ❌ |
| Go workspaces | ❌ |
| Poetry/uv workspaces | ❌ |

---

## Test Matrix: Trending Repos to Clone & Test

| Category | Repo Examples (search GitHub trending) | Expected Result |
|----------|----------------------------------------|-----------------|
| **React/Next.js** | `vercel/next.js`, `shadcn/ui`, `shadcn-ui` | Build + run |
| **Vue/Nuxt** | `nuxt/nuxt`, `vuejs/core` | Build |
| **Svelte/SvelteKit** | `sveltejs/kit`, `sveltejs/svelte` | Build |
| **Astro** | `withastro/astro` | Build |
| **Remix** | `remix-run/remix` | Build |
| **tRPC** | `trpc/trpc` | Build |
| **tRPC + Next.js** | `trpc/trpc-next` | Build |
| **Prisma** | `prisma/prisma` | Generate + build |
| **tRPC + Prisma + Next.js** | Common stack | Full build |
| **FastAPI** | `tiangolo/fastapi` | Build + run |
| **Django** | `django/django` | Build + run |
| **Streamlit** | `streamlit/streamlit` | Run |
| **Gradio** | `gradio-app/gradio` | Run |
| **LangChain** | `langchain-ai/langchain` | Build |
| **LlamaIndex** | `run-llama/llama_index` | Build |
| **Ollama** | `ollama/ollama` (Go) | ❌ Not supported |
| **vLLM** | `vllm-project/vllm` (Python) | Build |
| **Transformers** | `huggingface/transformers` (Python) | Build |
| **PyTorch** | `pytorch/pytorch` (C++/Python) | ❌ C++ |
| **TensorFlow** | `tensorflow/tensorflow` | ❌ C++/Bazel |
| **Rust CLI tools** | `rust-lang/rust`, `BurntSushi/ripgrep`, `sharkdp/fd` | ❌ Rust |
| **Go tools** | `golang/go`, `kubernetes/kubernetes`, `prometheus/prometheus` | ❌ Go |
| **Java/Spring** | `spring-projects/spring-boot` | ⚠️ No build |
| **Kotlin** | `JetBrains/kotlin` | ❌ |
| **Swift** | `apple/swift` | ❌ |
| **Zig** | `ziglang/zig` | ❌ |

---

## Immediate Action Items

### Phase 1: Quick Wins (Existing Runtime Edge Cases)
1. **pnpm workspaces** — Test `pnpm-workspace.yaml` + workspace protocol
2. **Yarn Berry** — Detect `yarnrc.yml` + `.yarn/` 
3. **Turborepo/Nx** — Detect `turbo.json` / `nx.json`
4. **Python uv workspaces** — Detect `uv.lock` + workspace config
5. **TypeScript project references** — Detect `tsconfig.json` references
6. **Private registries** — Parse `.npmrc` / `auth.json` / `pip.conf`
7. **Build scripts** — Run `npm run build` / `cargo build` / `go build` before payload

### Phase 2: New Runtime Support (Priority Order)
1. **Rust (Cargo)** — High demand, binary output
2. **Go (Modules)** — High demand, single binary output
3. **Java/Gradle/Maven** — Enterprise demand
4. **Rust + WASM** — `wasm-pack` + `wasm-bindgen`

### Phase 3: Advanced Features
1. Cross-compilation for Go/Rust
2. Private registry auth
2. Monorepo build orchestration (Turborepo/Nx/Cargo/Go)
3. SBOM generation

---

## Test Automation

```bash
# Quick test script
#!/bin/bash
# test_trending.sh
# Clone, build, verify

REPOS=(
  "https://github.com/vercel/next.js"           # Next.js
  "https://github.com/nuxt/nuxt"                # Nuxt 3
  "https://github.com/sveltejs/kit"             # SvelteKit
  "https://github.com/withastro/astro"          # Astro
  "https://github.com/tiangolo/fastapi"         # FastAPI
  "https://github.com/streamlit/streamlit"      # Streamlit
  "https://github.com/gradio-app/gradio"        # Gradio
  "https://github.com/langchain-ai/langchain"   # LangChain
  "https://github.com/prisma/prisma"            # Prisma
  "https://github.com/trpc/trpc"                # tRPC
)

for repo in "${REPOS[@]}"; do
  name=$(basename "$repo" .git)
  echo "Testing $name..."
  git clone --depth 1 "$repo" "/tmp/test-$name"
  cd "/tmp/test-$name"
  erebus build . -o "/tmp/$name.erebus" --verbose || echo "FAILED: $name"
  cd /
done
```

---

## Summary

**Currently erebus can build ~60% of trending repos** (Python, Node.js basic, PHP, Ruby, simple binaries).

**Missing for 100% coverage**: Rust, Go, Java, Swift, C/C++, private registries, monorepo build orchestration, cross-compilation for non-Node runtimes.

**Priority**: Add Rust + Go support first (high impact, binary output matches erebus model), then Java/Gradle, then cross-compile for all.