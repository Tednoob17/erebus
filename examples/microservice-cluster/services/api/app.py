"""Service API principal.

Expose une API REST protégée par le service d'authentification.
Écoute sur le port défini par ERE_API_PORT (défaut: 8000).
"""

import os
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
import json


class ApiHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "ok"}).encode())
        elif self.path == "/api/message":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"message": "Hello from API service!"}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass


def main():
    port = int(os.environ.get("ERE_API_PORT", "8000"))
    print(f"[api] service on http://127.0.0.1:{port}", flush=True)
    HTTPServer(("127.0.0.1", port), ApiHandler).serve_forever()


if __name__ == "__main__":
    main()
