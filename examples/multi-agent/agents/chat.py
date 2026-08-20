"""Agent de chat (chatbot simple).

Premier agent du binaire multi-agent. Répond aux questions de manière
conversationnelle.
"""

import os
import sys


def main():
    name = os.environ.get("AGENT_NAME", "chat")
    print(f"[{name}] agent de chat prêt (type 'quit' to exit)", flush=True)

    while True:
        try:
            msg = input("> ").strip()
        except EOFError:
            break

        if msg.lower() in ("quit", "exit", "q"):
            print("Au revoir!")
            break

        # Réponse simple sans modèle — pour un vrai agent, charger un GGUF
        if "bonjour" in msg.lower():
            print("Bonjour! Comment puis-je vous aider?")
        elif "merci" in msg.lower():
            print("Avec plaisir!")
        else:
            print("Je suis un chatbot simple. Demandez-moi quelque chose!")


if __name__ == "__main__":
    main()
