#!/usr/bin/env python3
import http.server
import socketserver
import sys

PORT = 8080

class COOPCOEPMiddleware(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Inject strict Cross-Origin Isolation headers required to unlock SharedArrayBuffer
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        # Prevent caching during development
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        super().end_headers()

def main():
    print("========================================")
    print("🛡️ Qualia-DB Secure Web Gateway")
    print(f"Starting isolated server on http://localhost:{PORT}")
    print("Injected Headers:")
    print("  - Cross-Origin-Opener-Policy: same-origin")
    print("  - Cross-Origin-Embedder-Policy: require-corp")
    print("========================================")
    print("Press Ctrl+C to stop.")

    try:
        with socketserver.TCPServer(("", PORT), COOPCOEPMiddleware) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server.")
        sys.exit(0)

if __name__ == '__main__':
    main()
