"""Agent IA minimal avec modèle locale + outils.

Démontre le packaging d'un agent IA (modèle GGUF + prompt + runtime Python)
dans un seul binaire erebus. Utilise llama-cpp-python pour l'inférence locale.

Usage:
  erebus build ./python-agent --artifact-type agent -o agent.erebus
  ./agent.erebus  # lance l'agent en mode chat interactif
"""

import os
import sys
import json


def format_prompt(messages: list[dict]) -> str:
    """Formate la conversation en prompt pour le tokenizer GGUF."""
    text = ""
    for msg in messages:
        role = msg.get("role", "user")
        content = msg.get("content", "")
        if role == "system":
            text += f"<s>[INST] <<SYS>>\\n{content}\\n<</SYS>>\\n"
        elif role == "user":
            text += f"{content} [/INST] "
        else:
            text += f"{content} </s>"
    return text


def main():
    model_path = os.environ.get("MODEL_PATH", "/app/model.gguf")

    if not os.path.exists(model_path):
        print(f"[agent] model not found at {model_path}", file=sys.stderr)
        print("[agent] build with: erebus build . --artifact-type agent -o agent.erebus")
        sys.exit(1)

    # LlamaCPP inference — lazy-loaded only when the agent is invoked
    from llama_cpp import Llama

    llm = Llama(
        model_path=model_path,
        n_ctx=2048,
        n_threads=os.cpu_count() or 4,
        verbose=False,
    )

    system_prompt = "You are a helpful coding assistant. Keep responses concise."
    messages = [{"role": "system", "content": system_prompt}]

    print(f"[agent] loaded model: {os.path.basename(model_path)}")
    print("[agent] ready (type 'quit' to exit)")

    while True:
        try:
            user_input = input("\\n> ").strip()
        except EOFError:
            break

        if user_input.lower() in ("quit", "exit", "q"):
            break

        messages.append({"role": "user", "content": user_input})
        prompt = format_prompt(messages)

        output = llm(
            prompt,
            max_tokens=512,
            stop=["</s>", "[INST]"],
            echo=False,
        )

        response = output["choices"][0]["text"].strip()
        messages.append({"role": "assistant", "content": response})
        print(f"\\n{response}")


if __name__ == "__main__":
    main()
