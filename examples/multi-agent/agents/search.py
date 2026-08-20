"""Agent de recherche (recherche web + synthèse).

Troisième agent du binaire multi-agent. Effectue des recherches et
synthétise les résultats.
"""

import os
import sys


def main():
    name = os.environ.get("AGENT_NAME", "search")
    print(f"[{name}] agent de recherche prêt (type 'quit' to exit)", flush=True)

    while True:
        try:
            msg = input("> ").strip()
        except EOFError:
            break

        if msg.lower() in ("quit", "exit", "q"):
            print("Au revoir!")
            break

        print(f"Recherche: {msg}")
        print("Résultats (simulation):")
        print("  - Article 1: 'Le développement IA en 2026'")
        print("  - Article 2: 'Les tendances Rust pour les systèmes embarqués'")
        print("  - Article 3: 'Le futur du packaging d'applications'")


if __name__ == "__main__":
    main()
