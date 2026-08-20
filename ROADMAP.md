# erebus — ROADMAP

## Vision

erebus est un **format d'artefact exécutable universel**, capable de transporter :

- une application classique ✅
- un agent IA (modèle + prompt + runtime) — objectif principal
- un service/microservice — objectif
- un plugin/extension — objectif

> Le format est conçu comme une unité autonome = stub + runtime + payload + metadata + signature.
> Rien n'est hérité du système hôte.

---

## Use case clé : Agent IA

Un agent IA est un binaire erebus qui contient :

```
[stub][runtime (python3)][model (gguf/bin)][prompt (toml)][tools (scripts/)][config]
```

**Ce que le format permet aujourd'hui :**
- Packager modèle + prompt + runtime dans un seul binaire portable
- Signature Ed25519 pour vérifier l'origine du modèle
- Chiffrement AES-256-GCM pour protéger les poids du modèle
- SISR pour update delta du modèle sans tout re-télécharger

**Ce qui manque (et pourquoi les couches sont nécessaires) :**
- **Hot-swap du modèle** : remplacer la couche `ModelLayer` (nouveaux poids) sans extraire le runtime, le prompt, ni les outils. Aujourd'hui le stub extrait tout le rootfs d'un coup — il faudrait extraire couche par couche et ne remplacer que la couche impactée.
- **Lazy loading** : ne charger le modèle (2-7 Go) que lorsqu'il est appelé, pas au démarrage du process. Le stub devrait pouvoir monter les couches à la demande via FUSE ou un mapping mémoire.
- **Multi-agent** : un seul binaire contenant 2-3 agents (chat, code, recherche) avec un entrypoint par agent. Le stub résout quel agent lancer selon les arguments.
- **Outils dynamiques** : les scripts/tools sont une couche séparée — on peut les mettre à jour (nouvelles APIs, nouveaux outils) sans toucher au modèle.

> Sans intégration des couches dans le stub (Phase 1), aucun de ces cas d'usage n'est possible.

---

## État actuel (août 2026)

### Couche cœur (`erebus-core`)
- ✅ CAS — `cas.rs` : `ObjectStore` trait, `MemoryStore`, `DiskObjectStore`
- ✅ Format binaire — `format.rs` : v2 (plain), v3 (signed), v4 (encrypted), v5 (squashfs)
- ✅ Metadata riche — `metadata.rs` : `ArtifactMetadata` avec `Vec<SerializableLayer>`
- ✅ Signature Ed25519 — `crypto.rs`
- ✅ Chiffrement AES-256-GCM — `encrypt.rs`
- ✅ Détection runtime — `detect.rs` : 13 runtimes, `EntrypointRegistry`
- ✅ Runtime Node.js — monorepo workspace sub-package scanning
- ✅ Runtime Ruby — Jekyll entrypoint resolution

### Abstraction des couches (`layer.rs`)
- ✅ Trait `Layer` : `name()`, `kind()`, `payload_sha256()`, `compression()`, `encryption()`, `capabilities()`
- ✅ Trait `Entrypoint` : `layer()`, `execute()`, `health_check()`
- ✅ `Capability` enum : `ReadFile`, `WriteFile`, `Network`, `Exec`, `Syscall`, `Env`
- ✅ `EntrypointRegistry` : registre dynamique de runtimes
- ✅ Types concrets : `RuntimeLayer`, `ModelLayer`, `ToolLayer`, `ConfigLayer`
- ✅ `LayerKind` : `Runtime`, `Model`, `Tool`, `Config`, `Custom`

### SISR / Delta updates (`sisr/`)
- ✅ `SisrEngine` : réutilisation de chunks, vérification SHA-256, swap atomique
- ✅ `HealthStore` : quarantaine, suivi des échecs
- ✅ `AtomicWriter` : remplacement atomique de fichiers
- ✅ Tests réseau avec injection de fautes
- ✅ **Phase 7 — Feature flags** : `--no-sisr` / `--no-sign` bypass dans le stub

### Assemblage
- ✅ `assemble_erebus` : builder unifié avec `AtomicWriter`, aware SISR
- ✅ Manifest distant : `.erebus.manifest`

### CLI (`erebus-cli`)
- ✅ `build`, `inspect`, `scan`, `sign`, `verify`, `keygen`, `trust`
- ✅ `doctor`, `env`, `clean`, `completion`, `man`, `upgrade`
- ✅ `swap` (hot-swap layers)
- ✅ `selftest`
- ✅ `build` module split : 2978 lignes → 8 modules (`args`, `deps`, `pipeline`, `payload`, `sign`, `sisr`, `stub`, `mod`)
- ✅ **Phase 6** : `--artifact-type` et `--entrypoint NAME=argv...` flags dans build
- ⚠️ `publish` — stub/placeholder, pas de vrai push/pull

### Stub launcher (`erebus-stub`)
- ✅ Lecture footer/metadata depuis `/proc/self/exe`
- ✅ Cache SHA-256, vérification d'intégrité
- ✅ Extraction zstd+tar ou squashfs
- ✅ `execvp` entrypoint
- ✅ SISR delta update
- ✅ Seccomp BPF — **capability-driven** (config dynamique depuis `layers[].capabilities`)
- ✅ Landlock LSM — **capability-driven** (R/W rootfs si `WriteFile`, read-only sinon)
- ✅ Health gate enforced — quarantaine + rollback automatique
- ✅ Signal handler async-signal-safe (static mut + AtomicUsize, pas Mutex)
- ✅ HTTP response 64 MiB limit (configurable via `ERE_HTTP_MAX_RESPONSE`)
- ✅ Zeroize pour toutes les clés crypto (ed25519, AES-256, HKDF)
- ✅ Clés privées en 0o600 (owner read/write only)
- ✅ **Phase 1 — Lit `ArtifactMetadata.layers`** — `Metadata` a `layers: Vec<SerializableLayer>` + `entrypoint_layer`
- ✅ **Phase 1 — Résolution entrypoint via couches** — `effective_entrypoint()`, `effective_runtime()`, `resolve_entrypoint()` utilisent les couches
- ✅ **Phase 5b — Capability-driven seccomp** — `SeccompConfig::from_capabilities()` → filtre BPF dynamique (Exec/Network/Syscall)
- ✅ **Phase 5b — Capability-driven landlock** — `LandlockConfig::from_capabilities()` → R/W rootfs si WriteFile
- ❌ N'utilise PAS le trait `Entrypoint` (a sa propre logique dans `exec.rs`) — couplage loose, phase future
- ❌ Pas de lazy loading — extrait tout le rootfs d'un coup — **Phase 3**
- ❌ Pas d'extraction couche par couche (rootfs monolithique) — **Phase 3**
- ✅ **Phase 6 — Multi-agent** : `entrypoints` map + `--erebus-agent` selector dans le stub
- ✅ **Phase 7 — Feature flags** : `--no-sisr` et `--no-sign` bypass (via `ERE_NO_SISR`/`ERE_NO_SIGN`)
- ✅ **Phase 7 — Error rescue** : pivot_root EPERM/EINVAL → fallback extract-only, seccomp ENOSYS → warning + continue

### Sécurité
- ✅ 35 findings review (17 true positives → tous fixés)
- ✅ ANSSI-Rust compliance check — zero unsafe dans erebus-core et erebus-cli
- ✅ PHP default bind 127.0.0.1 (pas 0.0.0.0), configurable via `ERE_PHP_HOST`
- ✅ **lru soundness fix** — ratatui `0.29 → 0.30.2` (pull lru `0.18.2` ≥ `0.16.3` patché)

### Build & DevX
- ✅ `build` module split : 2978 lignes → 8 modules sous `commands/build/`
- ✅ `.gitignore` fix : `build/` → `/build/` pour tracker le module directory

---

## Ce qui reste à faire

### Phase 1 : Intégrer les couches dans le stub ✅ FAIT

**Objectif** : Le stub doit consommer le système de couches, pas le contourner.
**Bloque** : agent IA, hot-swap, lazy loading, multi-agent, capabilities.

1. ✅ **Faire lire `ArtifactMetadata.layers` par le stub** — `Metadata` struct dans `main.rs` a `layers: Vec<SerializableLayer>` + `entrypoint_layer: Option<String>` avec `#[serde(default)]` pour backward compat
2. ✅ **Utiliser les couches pour résoudre l'entrypoint** — `resolve_entrypoint()` dans `exec.rs` extrait interpreter/argv du layer quand `entrypoint_layer` présent ; `exec_app()` utilise `meta.effective_entrypoint()`
3. ⏳ **Extraction couche par couche** — rootfs encore extrait en bloc ; extraction différée pour Phase 3 (lazy loading)
4. ⏳ **Supprimer l'ancien `Metadata` plat** — gardé pour backward compat ; migration complète quand plus de binaires legacy

> ⚠️ Phase 1 **partiellement terminée** : le stub lit et utilise les couches pour la résolution d'entrypoint, mais l'extraction reste monolithique (extrait tout le rootfs d'un coup). Le trait `Entrypoint` dans `erebus-core/src/layer.rs` n'est pas utilisé par le stub — il a sa propre logique dans `exec.rs`.

### Phase 2 : Hot-swap de couches ✅ FAIT

**Objectif** : Remplacer une couche sans rebuild complet du binaire.

1. ✅ **Commande `erebus swap <binary> <layer-name> <new-layer-file>`** — implémentée dans `erebus-cli/src/commands/swap.rs`
   - ✅ Lit footer + metadata depuis `/proc/self/exe`
   - ✅ Trouve layer cible par nom dans `metadata.layers`
   - ✅ Accepte fichier ou répertoire (auto-tar pour dirs)
   - ✅ Recalcule SHA-256, met à jour `layer.payload_sha256`
   - ✅ Ré-assemble via `AtomicWriter` (atomic rename, bit exécutable préservé)
2. ✅ **SISR-aware** : `--sisr-delta` génère delta update si binaire a SISR + clé de signature (`--key`)
3. ✅ **Use case agent** : `erebus swap agent.ere model ./nouveaux-poids.gguf` fonctionne

**Usage** :
```bash
erebus swap agent.ere model ./new-model.gguf
erebus swap agent.ere model ./new-model.gguf --sisr-delta --key ~/.erebus/keys/*.key
erebus swap agent.ere config ./new-config.toml -o agent-v2.ere
```

> Note: `--sisr-delta` génère un binaire complet avec SISR manifeste — le delta est calculé **à l'exécution** par le stub (chunk reuse), pas à la compilation. Le `swap` command ne compare pas la taille du delta vs le téléchargement complet.

### Phase 3 : Lazy loading (4-5 jours)

**Objectif** : Ne charger que les couches nécessaires, à la demande.

1. **Mapping mémoire des couches** — mmap le payload extrait, ne lit que les pages accédées
2. **Montage FUSE optionnel** — monter le rootfs sans extraction complète
3. **Use case agent** : le modèle (2-7 Go) n'est chargé en mémoire que quand l'agent reçoit un prompt, pas au démarrage du process
4. **CLI `erebus preload <binary> --layers model,config`** — pré-charger des couches spécifiques

### Phase 4 : Registry CAS (3-4 jours)

**Objectif** : Publier/charger des couches via un registre distant.

1. **Créer `Registry`** dans `erebus-core` au-dessus de `ObjectStore`
   - `pub fn push_layer(&self, layer: &dyn Layer) -> Result<String>` (retourne le hash)
   - `pub fn pull_layer(&self, hash: &str) -> Result<Box<dyn Layer>>`
   - `pub fn publish_artifact(&self, artifact: &ArtifactMetadata) -> Result<String>`
   - Transport : HTTP/HTTPS avec authentification token
2. **CLI `erebus registry push/pull/list`**
3. **Intégrer au build** : `erebus build --publish` pousse les couches automatiquement
4. **Use case agent** : partager le runtime python3 entre plusieurs agents (une seule couche runtime dans le registry, chaque agent référence le hash)

### Phase 5a : Security audit + enforcement de base ✅ FAIT

**Objectif** : Sécurité de base du stub. **Entièrement complété (août 2026).**

1. ✅ Seccomp BPF denylist — 18 syscalls dangereux bloqués (`seccomp.rs`)
2. ✅ Landlock LSM — sandbox rootfs R/W, lecture seule ailleurs (`landlock.rs`)
3. ✅ Health gate enforced — quarantaine + rollback automatique (`main.rs`)
4. ✅ Signal handler async-signal-safe (static mut + AtomicUsize)
5. ✅ HTTP response 64 MiB limit
6. ✅ Zeroize pour toutes les clés crypto
7. ✅ Clés privées en 0o600
8. ✅ 35 findings review (17 true positives → tous fixés)

### Phase 5b : Capability-driven security ✅ FAIT

**Objectif** : Les `Capability` doivent être vérifiées, pas juste déclarées. Le seccomp/landlock est maintenant capability-driven.

1. ✅ **Lire les capabilities depuis le binaire** — `Metadata::capabilities()` dans `main.rs:194` collecte `layers[].capabilities` (dédupliquées)
2. ✅ **Enforcer les capabilities dans le stub** :
   - `Exec` → seccomp allow `execve`/`execveat` (sinon deny)
   - `Network` → seccomp allow socket/connect/bind/listen/accept (sinon deny)
   - `Syscall` → seccomp allow dangerous syscalls (mount, pivot_root, kexec, etc.) (sinon deny)
   - `WriteFile` → landlock R/W rootfs (sinon read-only)
3. ✅ **Remplacer les denylists hardcoded** — `SeccompConfig::from_capabilities()` + `build_seccomp_filter()` dynamique (BPF construit selon caps) ; `LandlockConfig::from_capabilities()` + `sandbox()` config-driven
4. ✅ **Tests de sandboxing** — `seccomp.rs` tests: `validate_default_config_allows_exec_network_denies_syscalls`, `validate_no_capabilities_denies_all`, `validate_capability_config_mapping`

**Implémentation** :
- `erebus-stub/src/seccomp.rs` — `SeccompConfig`, `from_capabilities()`, `install_seccomp_filter()`, `build_seccomp_filter()`
- `erebus-stub/src/landlock.rs` — `LandlockConfig`, `from_capabilities()`, `sandbox(root, config)`
- `erebus-stub/src/exec.rs:605-624` — `enter_pivot_sandbox()` appelle les deux avec `meta.capabilities()`

> **Débloqué** — Phase 1 terminée, le stub lit `meta.layers` et applique les capabilities.

### Phase 6 : Templates + multi-agent (2-3 jours)

**Objectif** : Supporter les cas d'usage au-delà des applications.

1. **Templates de metadata** :
   - `application` — comportement actuel (un runtime, un entrypoint)
   - `agent` — modèle + prompt + runtime + toolcalls + capabilities réseau
   - `service` — multi-service avec orchestration
   - `plugin` — interface d'extension pour un hôte
2. **Multi-agent** : un seul binaire contenant plusieurs agents
   - `erebus build ./agents --entrypoint chat=chat.py --entrypoint code=code.py`
   - Le stub résout quel agent lancer selon `argv[1]`
   - Chaque agent a ses propres capabilities
3. **Exemples concrets dans `examples/`** :
   - `examples/python-agent/` — agent IA avec modèle locale + outils
   - `examples/multi-agent/` — 3 agents dans un seul binaire
   - `examples/microservice-cluster/` — 3 services avec orchestration
   - `examples/hugo-plugin/` — plugin Hugo statique
4. **Documentation** : guide de migration AppImage → erebus agents

---

## Priorité

1. ✅ **Phase 1** (stub → couches) — **FAIT**. `layers`, `entrypoint_layer`, `effective_entrypoint()`, `effective_runtime()`, `capabilities()`, `resolve_entrypoint()` consomment les couches
2. ✅ **Phase 2** (hot-swap) — **FAIT**. `erebus swap` command avec SISR delta
3. ✅ **Phase 5b** (capability-driven security) — **FAIT**. Seccomp + Landlock capability-driven (`from_capabilities()`, filtres dynamiques, tests)
4. ⏳ **Phase 3** (lazy loading) — **PROCHAINE**. Performance agent IA, mmap/FUSE, ne pas charger 7 Go au démarrage
5. ⏳ **Phase 4** (registry CAS) — **Adoption**. Partager couches entre artefacts
6. ⏳ **Phase 6** (templates + multi-agent) — **Storytelling**. Multi-agent, service, plugin
7. ⏳ **Phase 7** (hardening) — **Observability + rollback + risk mitigation**

---

## Single Points of Failure & Mitigations

| SPOF | Risk | Mitigation | Status |
|------|------|------------|--------|
| SISR manifest server (remote `.erebus.manifest`) | Network outage → updates fail permanently | Manifest fetched via HTTP with retry; stub reads its own embedded SISR manifest for chunk reuse context | ⚠️ Partiel |
| Cache corruption (`~/.cache/erebus/<hash>/`) | Corrupted cache → wrong behavior | `cache_root_trustworthy()` checks ownership + SHA-256 verification at cache check | ✅ Mitigé |
| CI musl cross-compilation | musl toolchain unavailable → no ARM builds | CI builds on x86_64 musl only; no multi-arch yet | ⚠️ A améliorer |
| Ed25519 signing key compromise | Attacker can forge binaries | `ed25519-dalek >= 2.1` enforces key clamping (CVE-2023-48022); key perms checked at build | ✅ Mitigé |

```
  SISR UPDATE FLOW
  ┌─────────────────┐
  │ User runs:      │
  │ ./myapp.ere    │
  └────────┬────────┘
           │
           ▼
  ┌──────────────────┐   NO   ┌──────────────┐
  │ Footer.has_sisr()?├───────►│ Classic      │
  └────────┬──────────┘        │ extraction   │
           │ YES               └──────────────┘
           ▼
  ┌──────────────────┐
  │ Fetch            │
  │ .erebus.manifest │
  └────────┬─────────┘
           │
    ┌──────▼──────┐
    │ SHA-256     │
    │ verify      │
    └──────┬──────┘
           │
  ┌────────▼────────┐   ┌─────────────────┐
  │ Chunk lookup    │◄──┤ Cache hits?     │
  │ in cache        │   └─────────────────┘
  └─────────────────┘
           │
           ▼
  ┌─────────────────┐
  │ Merge new+cached│
  │ chunks          │
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │ Verify SHA-256  │
  │ of merged       │
  │ payload         │
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │ Atomic swap to  │
  │ cache dir       │
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │ execvp entrypoint│
  └─────────────────┘
```

---

## Phase 7: Operational Hardening ⏳

**Objectif** : Passer de "fonctionne" à "fiable en production" avec observability, rollback, et documentation des flux critiques.

### Déjà implémenté ✅

- **Observabilé** : logging stderr (`[erebus] ...`) sur toutes les phases (extraction, vérification SISR, health gate, exec)
- **Health gate** : quarantaine + rollback automatique (`HealthStore`, `main.rs:390`)
- **Atomic swap** : `AtomicWriter` avec rollback snapshot (`main.rs:568`)
- **Signal handler** : async-signal-safe (`static mut + AtomicUsize`)
- **HTTP 64 MiB limit** : configurable via `ERE_HTTP_MAX_RESPONSE`
- **Zeroize** : toutes les clés crypto (ed25519, AES-256, HKDF)
- **Clé permissions** : 0o600 warning au build (`sign.rs:49`)
- **`erebus selftest`** : CLI command implémenté (`commands/selftest.rs`)
- **Feature flags `--no-sisr` / `--no-sign`** : bypass SISR update et signature verification (`main.rs`)
- **Error rescue map** : pivot_root EPERM/EINVAL fallback → extract-only (`exec.rs:672`), seccomp ENOSYS → warning + continue (`exec.rs:607`)
- **Rollback runbook** : `docs/runbooks/rollback-after-bad-deploy.md`

### Il manque ❌

1. **Observability structurée** : pas de métriques Prometheus, pas de `--metrics :9090`
2. **ASCII diagrams dans le code source** : pas de diagrams dans les sources
3. **Security audit deliverables** : `SECURITY-AUDIT.md` n'existe pas
4. **Chaos testing CLI** : tests de fault injection dans `sisr/network_test.rs` mais pas comme commandes CLI

---

## Phase 6: Templates + multi-agent ✅ IMPLÉMENTÉ

**Objectif** : Supporter les cas d'usage au-delà des applications.

### Déjà implémenté ✅

1. **Templates de metadata** : `ArtifactType` enum (`Application`, `Agent`, `Service`, `Plugin`) dans `erebus-core/src/metadata.rs`
   - `artifact_type` field in `ArtifactMetadata`
   - Default: `Application` (backward compatible)
   - Serialized as `artifact_type` in JSON

2. **Multi-agent** : entrypoints nommés + sélecteur `argv[1]`/`--erebus-agent`
   - `entrypoints: BTreeMap<String, Vec<String>>` in metadata
   - `multi_agent: bool` flag
   - Stub resolution: `select_agent_from_args()` → `--erebus-agent NAME` or `argv[1]`
   - Argv stripping: `ERE_AGENT_SELECTED` + `ERE_AGENT_NAME` env vars → `exec.rs` strips from forwarded argv
   - CLI `--entrypoint NAME=argv0,arg1,arg2` (repeatable) + `--artifact-type` flag

3. **Exemples concrets** (à créer) :
   - `examples/python-agent/`
   - `examples/multi-agent/`
   - `examples/microservice-cluster/`
   - `examples/hugo-plugin/`


---

## Contraintes techniques

- **vfat** : le repo vit sur vfat (pas de bit exec). Les artefacts build vont dans `/tmp/erebus-stub-target`.
- **musl** : le stub compile avec `--target x86_64-unknown-linux-musl` pour un ELF statique.
- **CI** : clippy est lancé par crate, pas workspace-wide.
- **Edition** : Rust 2021, stable uniquement (pas de nightly).
- **Sécurité** : zero `unsafe` dans `erebus-core` et `erebus-cli`. Toute la logique unsafe est dans `erebus-stub`.
