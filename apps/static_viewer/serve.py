#!/usr/bin/env python3
"""
OE1022D Dataset Viewer — static server.

Serves the current `runs/` directory as read-only HTTP, with a
small JSON API for the frontend to discover available runs.

Usage:
    python3 serve.py [--port 8000]

Open http://localhost:<port>/ in a browser to see the chart.
"""

import argparse
import json
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROJECT_ROOT = HERE.parent.parent  # apps/static_viewer/serve.py -> odmr-dataset-acquisition/
RUNS_DIR = PROJECT_ROOT / "runs"


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write(f"[serve] {self.address_string()} {fmt % args}\n")

    def do_GET(self):
        self.handle_request()

    def do_HEAD(self):
        self.handle_request(send_body=False)

    def handle_request(self, send_body=True):
        path = self.path.split("?", 1)[0]
        try:
            if path == "/" or path == "/index.html":
                self.serve_file(HERE / "index.html", "text/html", send_body=send_body)
            elif path == "/api/runs":
                if send_body:
                    self.serve_runs_list()
                else:
                    self.send_head_only("application/json")
            elif path.startswith("/api/runs/"):
                rel = path[len("/api/runs/"):]
                from urllib.parse import unquote
                rel = unquote(rel)
                if ".." in rel.split("/"):
                    self.send_error(400, "bad path")
                    return
                target = (RUNS_DIR / rel).resolve()
                if not str(target).startswith(str(RUNS_DIR.resolve())):
                    self.send_error(403, "forbidden")
                    return
                if not target.exists() or not target.is_file():
                    self.send_error(404, f"not found: {rel}")
                    return
                if target.suffix in (".ndjson", ".jsonl"):
                    ctype = "application/x-ndjson"
                else:
                    ctype = "text/plain"
                self.serve_file(target, ctype, send_body=send_body)
            else:
                self.send_error(404, f"unknown path: {path}")
        except (BrokenPipeError, ConnectionResetError):
            # Browser / curl closed the connection mid-stream; this
            # is normal for ndjson (which can be large). Don't
            # log as an error.
            sys.stderr.write(f"[serve] client closed: {path}\n")
        except Exception as e:
            sys.stderr.write(f"[serve] ERROR on {path}: {e}\n")
            try:
                self.send_error(500, str(e))
            except (BrokenPipeError, ConnectionResetError):
                pass

    def send_head_only(self, content_type: str):
        # 200 OK with Content-Type but no body.
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        self.end_headers()

    def serve_file(self, path: Path, content_type: str, send_body: bool = True):
        body = path.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", content_type + "; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        if send_body:
            self.wfile.write(body)

    def serve_runs_list(self):
        runs = []
        if RUNS_DIR.exists():
            for d in sorted(RUNS_DIR.iterdir()):
                if not d.is_dir():
                    continue
                f = d / "samples.ndjson"
                if not f.exists():
                    continue
                runs.append({
                    "run_id": d.name,
                    "size_bytes": f.stat().st_size,
                    "size_mb": f.stat().st_size / 1024 / 1024,
                })
        body = json.dumps(runs).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8000)
    ap.add_argument("--bind", default="127.0.0.1")
    args = ap.parse_args()

    if not RUNS_DIR.exists():
        print(f"[serve] WARNING: {RUNS_DIR} does not exist", file=sys.stderr)

    server = ThreadingHTTPServer((args.bind, args.port), Handler)
    print(f"[serve] serving {RUNS_DIR}")
    print(f"[serve] open http://{args.bind}:{args.port}/ in your browser")
    print(f"[serve] press Ctrl-C to stop")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[serve] shutting down")
        server.server_close()


if __name__ == "__main__":
    main()
