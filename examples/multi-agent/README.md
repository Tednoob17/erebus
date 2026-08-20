# Multi-Agent Example

A single erebus binary containing 3 AI agents (chat, code, research) with
runtime agent selection.

## Structure

```
multi-agent/
├── agents/
│   ├── chat.py        # Chatbot agent
│   ├── code.py        # Code assistant agent
│   └── search.py      # Research agent
└── pyproject.toml
```

## Build

```bash
# Build multi-agent binary with all 3 agents
erebus build ./multi-agent --artifact-type agent \
  --embed-interpreter python3 \
  --entrypoint chat=agents/chat.py \
  --entrypoint code=agents/code.py \
  --entrypoint search=agents/search.py \
  -o multi-agent.erebus

# The metadata will have:
# - artifact_type: "agent"
# - multi_agent: true
# - entrypoints: {"chat": ["python3", "agents/chat.py"], ...}
```

## Usage

```bash
# Select agent by name as argv[1]
./multi-agent.erebus chat
./multi-agent.erebus code
./multi-agent.erebus search

# Or via --erebus-agent flag
./multi-agent.erebus --erebus-agent code

# List available agents (no name specified)
./multi-agent.erebus
# → [erebus] available agents: chat, code, search
```

## How it works

1. The stub reads `meta.multi_agent == true` from metadata
2. It parses the agent name from `argv[1]` or `--erebus-agent <name>`
3. It selects the matching entrypoint from `meta.entrypoints`
4. It strips the agent name / `--erebus-agent` flag from argv before exec
5. The selected agent script runs

## Hot-swap a single agent

```bash
# Replace just the chat agent without rebuilding the whole binary
erebus swap multi-agent.erebus chat agents/new-chat.py
```
