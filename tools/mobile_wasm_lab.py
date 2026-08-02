#!/usr/bin/env python3
"""Serve the Qualia WASM LLM to a phone and capture bounded diagnostic events.

WebGPU is a secure-context API. A LAN IP over plain HTTP can exercise the UI and logger, but
model loading requires HTTPS with a certificate trusted by the phone. The easiest setup is a
certificate produced by mkcert after installing mkcert's local root CA on the phone.
"""

from __future__ import annotations

import argparse
import datetime as dt
import html
import http.server
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import sys
import threading
import urllib.parse


MAX_EVENT_BYTES = 64 * 1024
MAX_EVENTS = 20_000


def lan_ipv4() -> str:
    probe = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        probe.connect(("8.8.8.8", 80))
        return str(probe.getsockname()[0])
    except OSError:
        try:
            return socket.gethostbyname(socket.gethostname())
        except OSError:
            return "127.0.0.1"
    finally:
        probe.close()


def safe_session(value: str | None) -> str:
    if value and 8 <= len(value) <= 64 and all(c.isalnum() or c in "-_" for c in value):
        return value
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    return f"{stamp}-{secrets.token_hex(6)}"


class LabState:
    def __init__(
        self,
        docs: Path,
        artifact_root: Path,
        session: str,
        phone_url: str,
        ca_cert: Path | None,
        ca_url: str | None,
    ) -> None:
        self.docs = docs
        self.session = session
        self.phone_url = phone_url
        self.ca_cert = ca_cert
        self.ca_url = ca_url
        self.session_dir = artifact_root / session
        self.session_dir.mkdir(parents=True, exist_ok=False)
        self.events_path = self.session_dir / "events.jsonl"
        self.manifest_path = self.session_dir / "manifest.json"
        self.lock = threading.Lock()
        self.event_count = 0
        self.manifest_path.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "session": session,
                    "createdUtc": dt.datetime.now(dt.timezone.utc).isoformat(),
                    "phoneUrl": phone_url,
                    "events": str(self.events_path.resolve()),
                },
                indent=2,
            ),
            encoding="utf-8",
        )

    def append(self, remote: str, event: object) -> None:
        with self.lock:
            if self.event_count >= MAX_EVENTS:
                raise ValueError("event budget exhausted")
            record = {
                "serverReceivedUtc": dt.datetime.now(dt.timezone.utc).isoformat(),
                "remote": remote,
                "event": event,
            }
            encoded = json.dumps(record, ensure_ascii=False, separators=(",", ":"))
            if len(encoded.encode("utf-8")) > MAX_EVENT_BYTES:
                raise ValueError("event exceeds byte budget")
            with self.events_path.open("a", encoding="utf-8", newline="\n") as stream:
                stream.write(encoded + "\n")
            self.event_count += 1


class LabServer(http.server.ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, address: tuple[str, int], state: LabState):
        self.state = state
        super().__init__(address, LabHandler)


class LabHandler(http.server.SimpleHTTPRequestHandler):
    server: LabServer

    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "application/javascript",
        ".mjs": "application/javascript",
        ".json": "application/json",
        ".webmanifest": "application/manifest+json",
    }

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(args[2].state.docs), **kwargs)

    def end_headers(self) -> None:
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        if not self.path.startswith("/__qualia/lab"):
            self.send_header("Cross-Origin-Opener-Policy", "same-origin")
            self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
            self.send_header("Cross-Origin-Resource-Policy", "cross-origin")
        super().end_headers()

    def do_GET(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlsplit(self.path)
        if parsed.path == "/__qualia/lab":
            self._landing()
            return
        if parsed.path == "/__qualia/status":
            self._json(
                200,
                {
                    "ok": True,
                    "session": self.server.state.session,
                    "events": self.server.state.event_count,
                },
            )
            return
        if parsed.path == "/__qualia/root-ca.crt":
            self._root_ca()
            return
        super().do_GET()

    def do_POST(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlsplit(self.path)
        if parsed.path != "/__qualia/mobile-log":
            self.send_error(404)
            return
        query = urllib.parse.parse_qs(parsed.query)
        if query.get("lab", [""])[0] != self.server.state.session:
            self._json(403, {"ok": False, "error": "invalid session"})
            return
        try:
            size = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            size = 0
        if size <= 0 or size > MAX_EVENT_BYTES:
            self._json(413, {"ok": False, "error": "invalid event size"})
            return
        raw = self.rfile.read(size)
        try:
            event = json.loads(raw)
            self.server.state.append(self.client_address[0], event)
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError, OSError) as error:
            self._json(400, {"ok": False, "error": str(error)})
            return
        self._json(202, {"ok": True})

    def _landing(self) -> None:
        state = self.server.state
        encoded = urllib.parse.quote(state.phone_url, safe="")
        qr_url = f"https://api.qrserver.com/v1/create-qr-code/?size=280x280&data={encoded}"
        ca_step = ""
        if state.ca_cert is not None and state.ca_url is not None:
            ca_encoded = urllib.parse.quote(state.ca_url, safe="")
            ca_qr_url = f"https://api.qrserver.com/v1/create-qr-code/?size=220x220&data={ca_encoded}"
            ca_step = f"""<h2>1. Trust the lab certificate</h2>
<p>Scan this QR on the phone, or open the link below.</p>
<p><img src="{html.escape(ca_qr_url)}" width="220" height="220" alt="QR code for the public lab CA certificate"></p>
<p><a href="{html.escape(state.ca_url)}">Download the public Qualia mobile-lab CA certificate</a>,
then install it as a CA certificate in the phone's security settings. Return here and scan the QR.
Only the public certificate is downloadable; the private key remains on this computer.</p>"""
        body = f"""<!doctype html><meta charset=utf-8><title>Qualia mobile WASM lab</title>
<style>body{{font:16px system-ui;max-width:720px;margin:3rem auto;padding:0 1rem;background:#0b1020;color:#e5e7eb}}code{{word-break:break-all}}img{{background:white;padding:12px;border-radius:12px}}a{{color:#67e8f9}}</style>
<h1>Qualia mobile WASM lab</h1>
{ca_step}
<h2>{"2. Open the HTTPS model app" if state.ca_cert is not None else "Open the model app"}</h2>
<p>Scan this on the phone, or copy the URL. The QR image is rendered by api.qrserver.com; the URL below remains usable if it is blocked.</p>
<p><img src="{html.escape(qr_url)}" width="280" height="280" alt="QR code for phone lab URL"></p>
<p><a href="{html.escape(state.phone_url)}"><code>{html.escape(state.phone_url)}</code></a></p>
<p>Session <code>{html.escape(state.session)}</code>. Events are stored locally and capped at {MAX_EVENTS:,} records / {MAX_EVENT_BYTES // 1024} KiB each.</p>"""
        encoded_body = body.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded_body)))
        self.end_headers()
        self.wfile.write(encoded_body)

    def _root_ca(self) -> None:
        ca_cert = self.server.state.ca_cert
        if ca_cert is None:
            self.send_error(404)
            return
        try:
            body = ca_cert.read_bytes()
        except OSError as error:
            self.send_error(500, str(error))
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/x-x509-ca-cert")
        self.send_header("Content-Disposition", 'attachment; filename="qualia-mobile-lab-rootCA.crt"')
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _json(self, status: int, value: object) -> None:
        body = json.dumps(value, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args) -> None:
        if self.command == "POST" or (args and any(code in str(args[0]) for code in ("400", "403", "404", "413", "500"))):
            super().log_message(fmt, *args)


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", type=int, default=8443)
    parser.add_argument("--lan-ip", default=lan_ipv4())
    parser.add_argument("--cert", type=Path, help="PEM certificate with the LAN IP in its SAN")
    parser.add_argument("--key", type=Path, help="PEM private key for --cert")
    parser.add_argument("--allow-insecure", action="store_true", help="serve HTTP (logging/UI only; LAN WebGPU will be unavailable)")
    parser.add_argument("--docs", type=Path, default=root / "docs")
    parser.add_argument("--artifacts", type=Path, default=root / ".qualia" / "mobile-wasm-lab")
    parser.add_argument("--session")
    parser.add_argument("--phone-url", help="override the QR/link target (useful for an HTTP certificate-bootstrap page)")
    parser.add_argument("--ca-cert", type=Path, help="public CA certificate offered by the landing page")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    use_tls = args.cert is not None or args.key is not None
    if use_tls and (args.cert is None or args.key is None):
        raise SystemExit("--cert and --key must be provided together")
    if not use_tls and not args.allow_insecure:
        print("LAN WebGPU requires trusted HTTPS. Provide --cert/--key, or use --allow-insecure for a logging/UI-only probe.", file=sys.stderr)
        print(f"mkcert example: mkcert -cert-file lan.pem -key-file lan-key.pem {args.lan_ip} localhost 127.0.0.1", file=sys.stderr)
        return 2
    if use_tls and (not args.cert.is_file() or not args.key.is_file()):
        raise SystemExit("certificate or key file does not exist")
    if args.ca_cert is not None and not args.ca_cert.is_file():
        raise SystemExit("CA certificate file does not exist")

    scheme = "https" if use_tls else "http"
    session = safe_session(args.session)
    phone_url = args.phone_url or f"{scheme}://{args.lan_ip}:{args.port}/online-llm-demo.html?lab={session}&labText=1"
    ca_cert = args.ca_cert.resolve() if args.ca_cert is not None else None
    ca_url = f"{scheme}://{args.lan_ip}:{args.port}/__qualia/root-ca.crt" if ca_cert is not None else None
    state = LabState(args.docs.resolve(), args.artifacts.resolve(), session, phone_url, ca_cert, ca_url)
    server = LabServer((args.host, args.port), state)
    if use_tls:
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        context.load_cert_chain(str(args.cert.resolve()), str(args.key.resolve()))
        server.socket = context.wrap_socket(server.socket, server_side=True)

    local_url = f"{scheme}://localhost:{args.port}/__qualia/lab"
    print(f"Qualia mobile WASM lab session: {session}")
    print(f"Open on this computer (QR): {local_url}")
    print(f"Open on phone:              {phone_url}")
    print(f"Telemetry:                  {state.events_path.resolve()}")
    if not use_tls:
        print("WARNING: insecure LAN HTTP is not a WebGPU secure context; this run can only diagnose boot/UI/network behavior.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
