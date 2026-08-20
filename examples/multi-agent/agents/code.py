"""Agent de code (assistant de programmation).

Deuxième agent du binaire multi-agent. Fournit des suggestions de code
et des explications techniques.
"""

import os
import sys


def main():
    name = os.environ.get("AGENT_NAME", "code")
    print(f"[{name}] agent de code prêt (type 'quit' to exit)", flush=True)

    while True:
        try:
            msg = input("> ").strip()
        except EOFError:
            break

        if msg.lower() in ("quit", "exit", "q"):
            print("Au revoir!")
            break

        if "python" in msg.lower() or "code" in msg.lower():
            print("```python")
            print("# Exemple de code généré:")
            print(f"# Request: {msg}")
            print("print('Hello, World!')")
            print("```")
        elif "rust" in msg.lower():
            print("```rust")
            print("fn main() {")
            print("    println!(\"Hello, World!\");")
            print("}")
            print("```")
        else:
            print("Je peux vous aider avec du code Python ou Rust. Quel langage?")


if __name__ == "__main__":
    main()
