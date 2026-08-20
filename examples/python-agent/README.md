# Python Agent Example

An AI coding agent packaged as a single erebus binary.

## Structure

```
python-agent/
├── agent.py           # Main agent entrypoint (chat interface)
├── model.gguf         # GGUF model weights (symlink or copy)
├── requirements.txt   # Python dependencies (llama-cpp-python)
└── pyproject.toml     # Project metadata + erebus config
```

## Build

```bash
# Download a small GGUF model (e.g. Qwen3-0.6B)
huggingface-cli download unsloth/Qwen3-0.6B-GGUF --filename qwen3-0.6b-gguf-q4_k_m.gguf --local-dir model

# Build the agent binary
erebus build ./python-agent --artifact-type agent \
  --embed-interpreter python3 \
  --include ./model/model.gguf \
  -o python-agent.erebus

# Run
./python-agent.erebus
```

## Multi-agent usage

This example can also be part of a multi-agent binary:

```bash
erebus build ./python-agent --artifact-type agent \
  --entrypoint chat=/app/agent.py \
  --entrypoint code=/app/code_agent.py \
  -o multi-agent.erebus

# Select agent at runtime
./multi-agent.erebus chat
./multi-agent.erebus code
# or
./multi-agent.erebus --erebus-agent chat
```

## Hot-swap model

```bash
# Swap in new model weights without rebuilding the entire binary
erebus swap python-agent.erebus model ./new-model.gguf --sisr-delta --key ~/.erebus/keys/*.key
```
