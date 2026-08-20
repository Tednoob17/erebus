"""Plugin Hugo minimal.

Un plugin qui transforme le texte en majuscules (exemple simple de transformation).
Hugo charge ce plugin via `hugo --plugin hugo-uppercase`.
"""

import sys
import os


def process(text: str) -> str:
    """Transforme le texte en majuscules + emoji decoration."""
    return text.upper()


def main():
    """Hugo plugin entrypoint — reads stdin, writes stdout."""
    data = sys.stdin.read()
    result = process(data)
    sys.stdout.write(result)


if __name__ == "__main__":
    main()
