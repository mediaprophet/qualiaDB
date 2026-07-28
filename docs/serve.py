#!/usr/bin/env python3
"""Static file server with correct MIME types and COOP/COEP headers for WASM + SharedArrayBuffer."""
import http.server
import socketserver
import os

class QualiaHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        '.wasm': 'application/wasm',
        '.js': 'application/javascript',
        '.mjs': 'application/javascript',
        '.json': 'application/json',
        '.webmanifest': 'application/manifest+json',
    }

    def end_headers(self):
        # Enable cross-origin isolation for SharedArrayBuffer
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Resource-Policy', 'cross-origin')
        super().end_headers()

    def log_message(self, format, *args):
        # Suppress noisy logs except errors
        if args and '404' in str(args[0]):
            super().log_message(format, *args)

if __name__ == '__main__':
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    PORT = 8000
    with socketserver.TCPServer(('0.0.0.0', PORT), QualiaHandler) as httpd:
        print(f'Serving docs/ on http://localhost:{PORT} with COOP/COEP headers')
        httpd.serve_forever()
