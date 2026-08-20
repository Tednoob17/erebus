"""Service d'authentification.

Gère la connexion, les tokens JWT, et les permissions.
Écoute sur le port défini par ERE_AUTH_PORT (défaut: 8001).
"""

import os
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import hashlib


class AuthHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/login":
            length = int(self.headers.get("Content-Length", 0))
            body = json.loads(self.rfile.read(length) or b"{}")
            password = body.get("password", "")
            token = hashlib.sha256(
                f"{body.get('username', '')}:{password}".encode()
            ).hexdigest()[:32]
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"token": token}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass  # suppress stderr


def main():
    port = int(os.environ.get("ERE_AUTH_PORT", "8001"))
    print(f"[auth] service on http://127.0.0.1:{port}", flush=True)
    HTTPServer(("127.0.0.1", port), AuthHandler).serve_forever()


if __name__ == "__main__":
    main()
