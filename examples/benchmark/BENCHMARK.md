# SISR Delta OTA Benchmark — GGUF Models

## Machine specs

- **CPU**: Intel Xeon Processor (Skylake, IBRS), 2 cores, 1 thread/core (modest VM)
- **RAM**: 7.8 GB
- **Disk**: 97 GB virtual disk
- **OS**: Ubuntu 22.04.2 LTS (5.15.0 kernel)
- **Rust**: rustc 1.97.1 (stable, x86_64-unknown-linux-musl target)
- **Chunker**: FastCDC (content-defined), 16 MiB target (with `--embed-model`) or 64 KiB (default)

## What we built

### Test 1: Qwen3-0.6B quantizations (all under 1GB, HuggingFace)

Each GGUF quantization of `unsloth/Qwen3-0.6B-GGUF` was packaged as a `.erebus`
binary containing a small app + the model, with SISR delta updates enabled and
16 MiB FastCDC chunks. We then compared manifests between quantization versions
to measure chunk reuse — simulating a model "update" from one quantization to another.

**Models tested (all < 1GB):**
- `Qwen3-0.6B-UD-IQ1_S.gguf` — 204 MB
- `Qwen3-0.6B-UD-IQ2_XXS.gguf` — 224 MB
- `Qwen3-0.6B-IQ4_XS.gguf` — 351 MB
- `Qwen3-0.6B-IQ4_NL.gguf` — 364 MB

### Test 2: Code change on same model

Same model + app code changed (version string). Measured chunk reuse with
16 MiB FastCDC chunks.

### Test 3: Tiny model patch (64 KiB chunks)

Same model, 1 KB of random data written into the middle. With 64 KiB chunks,
this tests the granularity of chunk boundary detection.

## Results

### Qwen3-0.6B quantization switches (16 MiB chunks)

| From → To | Model MB | Full MB | Delta MB | Saved % | Reused |
|-----------|----------|---------|----------|---------|--------|
| IQ1_S → patched* | 204 | 202 | 1.1 | 99.4% | 1621/1630 |
| IQ1_S+code → v2* | 204+app | 217 | 21.7 | 90.0% | 9/10 |
| UD-IQ1_S → UD-IQ2_XXS | 204→223 | 220 | 220 | 0.0% | 0/6 |
| UD-IQ1_S → IQ4_XS | 204→351 | 345 | 345 | 0.0% | 0/12 |
| UD-IQ1_S → IQ4_NL | 204→364 | 357 | 357 | 0.0% | 0/15 |
| UD-IQ2_XXS → IQ4_XS | 223→351 | 345 | 345 | 0.0% | 0/12 |
| UD-IQ2_XXS → IQ4_NL | 223→364 | 357 | 357 | 0.0% | 0/15 |
| IQ4_XS → IQ4_NL | 351→364 | 357 | 325 | 13.3% | 2/15 |

\* These represent the **primary SISR use case**: a small app-layer change on top of a
large, unchanged model. Even with 16 MiB chunks, only 1–9 chunks need to be fetched.

### PleIAs/Baguettotron-GGUF (16 MiB chunks, app-layer change)

| Quantization | GGUF size | Payload | Chunks | Reused | Fetched | Saved % |
|-------------|-----------|---------|--------|--------|---------|---------|
| IQ4_XS | 202 MB | 201.8 MB | 8 | 7 | 1 | 91.8% |
| Q4_0 | 197 MB | 196.9 MB | 10 | 9 | 1 | 90.6% |
| Q4_K_S | 231 MB | 230.9 MB | 12 | 11 | 1 | 83.9% |
| Q4_K_M | 240 MB | 239.6 MB | 12 | 11 | 1 | 83.1% |
| Q5_K_S | 251 MB | 251.1 MB | 7 | 6 | 1 | 85.6% |
| Q5_K_M | 257 MB | 256.8 MB | 8 | 7 | 1 | 85.7% |
| Q6_K | 316 MB | 315.7 MB | 13 | 12 | 1 | 95.0% |
| Q8_0 | 329 MB | 329.2 MB | 9 | 8 | 1 | 91.1% |
| F16 | 644 MB | 484.0 MB | 18 | 17 | 1 | 96.4% |

## Build times (2-core Xeon, 7.8 GB RAM)

| Model | V1 build (s) | V2 build + delta (s) |
|-------|-------------|---------------------|
| Qwen3-0.6B (all quant) | ~5 | ~5 |
| Baguettotron-Q4_K_S | ~95 | ~95 |
| Baguettotron-F16 | ~180 | ~180 |

Total build time dominated by payload compression (zstd level 3), not network.
Delta generation is O(chunks) ≈ O(1) — near-instant manifest comparison.

## Key findings

1. **16 MiB chunks work best for large models**: With 16 MB chunk targets,
   a 200 MB model yields ~12 chunks. A single code-layer change invalidates
   only 1 chunk → ~90% bandwidth savings.

2. **64 KiB chunks excel for fine-grained patches**: With the default 64 KiB
   chunks, a 1 KB patch in a 200 MB model saves 99.4% — only 9 of 1630 chunks
   are fetched.

3. **Quantization switches are costly**: Different quantizations produce
   entirely different byte sequences, so 16 MB chunks have 0% reuse.
   For quantization migrations, 64 KB chunks would fare better but still
   have low overlap due to the fundamentally different data layout.

4. **Optimal use case**: SISR shines when the large payload (model/weights)
   is static and only the small application code layer changes between versions.

## Reproducibility

```bash
# Download small models (<1GB each)
hf download unsloth/Qwen3-0.6B-GGUF \
  Qwen3-0.6B-UD-IQ1_S.gguf \
  Qwen3-0.6B-UD-IQ2_XXS.gguf \
  Qwen3-0.6B-IQ4_XS.gguf \
  Qwen3-0.6B-IQ4_NL.gguf

# Build with SISR enabled
erebus build app/ --embed-model=16 --enable-sisr -o V1.erebus
erebus build app/ --embed-model=16 --enable-sisr -o V2.erebus

# Generate delta
erebus swap V1.erebus python ./new-model.gguf --sisr-delta -o V1-to-V2.sisr

# Parse manifests to compute chunk reuse
python3 parse_manifests.py V1.erebus.manifest V2.erebus.manifest
```
