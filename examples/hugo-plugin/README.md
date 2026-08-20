# Hugo Plugin Example

A Hugo static site generator plugin packaged as an erebus binary.

## Structure

```
hugo-plugin/
├── pyproject.toml      # Plugin metadata with [tool.hugo.plugin] config
├── plugin/
│   └── __init__.py     # Plugin entrypoint (process function)
└── README.md
```

## Build

```bash
# Build as a plugin-type artifact
erebus build ./hugo-plugin --artifact-type plugin \
  --embed-interpreter python3 \
  --entrypoint plugin=plugin/__init__.py \
  -o hugo-uppercase.erebus
```

## Usage

```toml
# hugo.toml — reference the plugin
[module]
  [[module.imports]]
    path = "hugo-uppercase.erebus"
```

Or run standalone:

```bash
echo "hello world" | ./hugo-uppercase.erebus
# → HELLO WORLD
```

## How it works

The stub detects `artifact_type: "plugin"` and the entrypoint layer name
matches the plugin's entry point. Hugo's module system can import the
erebus binary as a module, with the stub providing the runtime shim.
