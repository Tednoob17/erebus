"""Service worker (background tasks).

Traite des tâches en arrière-plan (traitement async).
"""

import os
import sys
import time
import json


def main():
    interval = int(os.environ.get("ERE_WORKER_INTERVAL", "5"))
    print(f"[worker] starting with {interval}s interval", flush=True)

    iteration = 0
    while True:
        iteration += 1
        print(f"[worker] iteration {iteration} — processed batch", flush=True)
        time.sleep(interval)


if __name__ == "__main__":
    main()
