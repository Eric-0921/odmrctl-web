#!/usr/bin/env python3
"""
OE1022D Dataset Viewer — static server with SSE push.

Serves the current `runs/` directory as read-only HTTP, with a
small JSON API for the frontend to discover available runs
and a Server-Sent-Events (SSE) endpoint for live chart push.

Modes:
- HTTP `GET /api/stream/<run_id>?speed=N` opens an SSE
  stream that emits a `chart` event every 16 ms (~60 Hz)
  containing the chart window. The server reads the
  run's samples.ndjson file and replays it at the
  configured speed. v0.2 will swap this replay loop for a
  real Rust SSE server that pushes live streaming samples
  from the 1 kHz acquisition pipeline; the frontend
  EventSource contract stays the same.

Usage:
    python3 serve.py [--port 8000]

Open http://localhost:<port>/ in a browser to see the chart.
"""

import argparse
import json
import os
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, parse_qs

HERE = Path(__file__).resolve().parent
PROJECT_ROOT = HERE.parent.parent  # apps/static_viewer/serve.py -> odmr-dataset-acquisition/
RUNS_DIR = PROJECT_ROOT / "runs"


def parse_ndjson(path: Path):
    """Stream a samples.ndjson file and yield per-line dicts.

    The file is large (5 min × 1 kHz × 3 fields ≈ 1 M lines),
    so we yield lazily and let the SSE handler pace the read.
    """
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                # Skip malformed lines (warmup garbage, etc.).
                continue


class Handler(BaseHTTPRequestHandler):
    # Disable the default per-request logging for the SSE path
    # because it would log one line per ~16ms tick. We log only
    # at connection / error granularity.
    def log_message(self, fmt, *args):
        if "GET /api/stream" in (fmt % args) or "POST" in (fmt % args):
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
            elif path.startswith("/api/stream/"):
                self.serve_sse_stream(path)
            elif path.startswith("/api/runs/"):
                rel = path[len("/api/runs/"):]
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
            sys.stderr.write(f"[serve] client closed: {path}\n")
        except Exception as e:
            sys.stderr.write(f"[serve] ERROR on {path}: {e}\n")
            try:
                self.send_error(500, str(e))
            except (BrokenPipeError, ConnectionResetError):
                pass

    def send_head_only(self, content_type: str):
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

    def serve_sse_stream(self, path: str):
        """Server-Sent-Events stream of `chart` events.

        URL: /api/stream/<run_id>?speed=N&window=W&emit_ms=T

        speed: how many real-time seconds of data to emit per
               wall-clock second (default 60; >1 = fast forward)
        window: how many samples to include in each chart event
                (default 1000)
        emit_ms: how often to push a chart event in milliseconds
                 (default 16 = ~60 Hz)

        The first `chart` event includes a `meta` payload with
        field info; subsequent events are pure data.
        """
        # Parse path: /api/stream/<run_id>
        rel = path[len("/api/stream/"):]
        rel = unquote(rel)
        if ".." in rel.split("/") or not rel:
            self.send_error(400, "bad run_id")
            return
        ndjson_path = (RUNS_DIR / rel / "samples.ndjson").resolve()
        if not str(ndjson_path).startswith(str(RUNS_DIR.resolve())):
            self.send_error(403, "forbidden")
            return
        if not ndjson_path.exists():
            self.send_error(404, f"run not found: {rel}")
            return

        # Query params.
        qs = parse_qs(self.path.split("?", 1)[1] if "?" in self.path else "")
        speed = float(qs.get("speed", ["60"])[0])
        window = int(qs.get("window", ["1000"])[0])
        emit_ms = int(qs.get("emit_ms", ["16"])[0])
        emit_seconds = emit_ms / 1000.0
        # Per emit, advance the playhead by `speed * emit_seconds`
        # samples. For 60 Hz and 60x speed that's 60 samples per
        # emit; for 1 kHz × 1x that's 16 samples per emit.
        advance_per_emit = max(1, int(speed * emit_seconds))

        # SSE response headers.
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Connection", "keep-alive")
        self.send_header("Access-Control-Allow-Origin", "*")
        # Disable proxy buffering.
        self.send_header("X-Accel-Buffering", "no")
        self.end_headers()

        # First event: meta with the run info.
        meta = {
            "type": "meta",
            "run_id": rel,
            "ndjson_path": str(ndjson_path.relative_to(PROJECT_ROOT)),
            "speed": speed,
            "window": window,
            "emit_ms": emit_ms,
            "advance_per_emit": advance_per_emit,
        }
        self._sse_send({"event": "meta", "data": json.dumps(meta)})

        # Read ndjson lazily. We hold an index over samples; each
        # emit advances the index by advance_per_emit and
        # builds a chart window of the last `window` samples.
        sample_iter = parse_ndjson(ndjson_path)
        # Buffer of the most recent samples we have seen.
        recent = []  # list of (t_mono_ns, t_wall_ms, field, value)
        next_idx = 0  # next sample we want to push to client
        last_emit_ts = time.monotonic()
        last_heartbeat = time.monotonic()
        # How often to send a "still alive" comment.
        heartbeat_seconds = 5.0

        try:
            while True:
                # Drain the iterator up to the next index we
                # want to emit.
                while len(recent) < next_idx + advance_per_emit:
                    try:
                        s = next(sample_iter)
                    except StopIteration:
                        # End of file. We could keep streaming
                        # the last window or close. For v0.1 we
                        # close after sending the final "eof" event
                        # so the frontend knows.
                        self._sse_send({"event": "eof", "data": json.dumps({"run_id": rel, "total_samples": next_idx})})
                        return
                    recent.append((s.get("t_mono_ns", 0), s.get("t_wall_ms", 0), s.get("field", "?"), s.get("value", 0.0)))
                # Trim window.
                if len(recent) > window:
                    recent = recent[-window:]
                # Build chart window payload: per field, list of
                # {t, y}.
                by_field = {}
                for (t_mono, t_wall, f, v) in recent:
                    by_field.setdefault(f, []).append({"t": t_mono, "y": v})
                payload = {
                    "type": "chart",
                    "run_id": rel,
                    "next_idx": next_idx + advance_per_emit,
                    "by_field": by_field,
                }
                self._sse_send({"event": "chart", "data": json.dumps(payload)})
                next_idx += advance_per_emit

                # Pace to ~60 Hz: sleep until the next emit slot.
                now = time.monotonic()
                elapsed = now - last_emit_ts
                if elapsed < emit_seconds:
                    time.sleep(emit_seconds - elapsed)
                last_emit_ts = time.monotonic()

                # Periodic heartbeat so proxies / load balancers
                # don't kill the connection.
                if last_emit_ts - last_heartbeat > heartbeat_seconds:
                    self.wfile.write(b": heartbeat\n\n")
                    self.wfile.flush()
                    last_heartbeat = last_emit_ts
        except (BrokenPipeError, ConnectionResetError):
            sys.stderr.write(f"[serve] SSE client disconnected: {rel}\n")
            return

    def _sse_send(self, msg):
        """Send one SSE message. The msg dict must have `event`
        and `data` keys. We use raw bytes writing for speed
        (60 events/s on a 5-min dataset is 18k events)."""
        line = f"event: {msg['event']}\ndata: {msg['data']}\n\n"
        self.wfile.write(line.encode("utf-8"))
        self.wfile.flush()


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
    print(f"[serve] SSE: /api/stream/<run_id>?speed=60&window=1000&emit_ms=16")
    print(f"[serve] press Ctrl-C to stop")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[serve] shutting down")
        server.server_close()


if __name__ == "__main__":
    main()
