#!/usr/bin/env python3
"""
Batch AU federal legislation PDFs → CML/COF packages (ns.webcivics-style).

Default corpus:
  C:\\Users\\Admin\\Downloads\\20260630_AU-FED-LEGISLATION

Uses local Ollama (optional --no-llm for structure-only smoke). Content-addressed,
provision-aligned segment resume lets multi-day runs survive restarts safely.

Examples:
  python batch_au_legislation.py --limit 2 --no-llm
  python batch_au_legislation.py --max-segment-items 3 --emit-ttl
  python batch_au_legislation.py --resume --ollama-url http://127.0.0.1:11434
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import threading
from collections import deque
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
LEGIS2CML = HERE / "legis2cml.py"
DEFAULT_IN = Path(r"C:\Users\Admin\Downloads\20260630_AU-FED-LEGISLATION")
DEFAULT_OUT = Path(r"C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation")


def run_child(cmd: list[str], prefix: str, timeout: int) -> tuple[int | None, str]:
    """Run legis2cml, streaming its output live while keeping a bounded tail for the log.

    ``capture_output`` buffered everything until the child exited, so a large Act showed no
    output for many minutes and read as a hang. Streaming surfaces real per-segment progress.
    An optional wall-clock timeout kills a genuinely stuck child; segment checkpoints are
    atomic, so ``--resume`` recovers cleanly. Ctrl+C terminates the child and propagates.
    """
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            encoding="utf-8", errors="replace", bufsize=1)
    tail: deque[str] = deque(maxlen=40)
    timed_out = False
    timer: threading.Timer | None = None
    if timeout and timeout > 0:
        def _kill() -> None:
            nonlocal timed_out
            timed_out = True
            proc.kill()
        timer = threading.Timer(timeout, _kill)
        timer.start()
    try:
        for line in proc.stdout:  # blocks until the child closes stdout (exit or kill)
            line = line.rstrip("\n")
            if line:
                tail.append(line)
                print(f"{prefix}{line}", flush=True)
        proc.wait()
    except KeyboardInterrupt:
        proc.kill()
        proc.wait()
        raise
    finally:
        if timer is not None:
            timer.cancel()
    rc = proc.returncode
    if timed_out:
        note = f"[batch] killed after {timeout}s timeout (rerun with --resume)"
        tail.append(note)
        print(f"{prefix}{note}", flush=True)
        if rc == 0:
            rc = -9
    return rc, "\n".join(tail)


def main() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except (AttributeError, OSError):
        pass
    ap = argparse.ArgumentParser(description="Batch AU legislation → CML/COF packages")
    ap.add_argument("--input-dir", type=Path, default=DEFAULT_IN)
    ap.add_argument("--out-root", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--model", default="llama3.2:3b-instruct-q4_K_M")
    ap.add_argument("--ollama-url", default="http://127.0.0.1:11434")
    ap.add_argument("--limit", type=int, default=0, help="process only first N PDFs (0 = all)")
    ap.add_argument("--no-llm", action="store_true")
    ap.add_argument("--resume", action="store_true", help="pass --resume to legis2cml")
    ap.add_argument("--link", action="store_true", help="do not copy PDFs into packages")
    ap.add_argument("--max-segment-chars", type=int, default=8000)
    ap.add_argument("--max-segment-items", type=int, default=1)
    ap.add_argument("--segment-overlap-chars", type=int, default=500)
    ap.add_argument("--ollama-timeout", type=int, default=180)
    ap.add_argument("--max-retries", type=int, default=2)
    ap.add_argument("--file-timeout", type=int, default=0,
                    help="seconds per PDF before the child is killed (0 = no limit; resume-safe)")
    ap.add_argument("--emit-ttl", action="store_true", help="validate JSON-LD and emit Turtle")
    ap.add_argument("--emit-q42", action="store_true", help="compile and round-trip verify with QualiaDB")
    ap.add_argument("--qualia-cli", type=Path, default=None)
    ap.add_argument("--q42-shard-provisions", type=int, default=40)
    ap.add_argument("--jurisdiction", default="AU")
    ap.add_argument("--base-iri-prefix", default="https://ns.webcivics.net/values/au/")
    args = ap.parse_args()

    if not LEGIS2CML.is_file():
        sys.exit(f"missing {LEGIS2CML}")
    if not args.input_dir.is_dir():
        sys.exit(f"input dir not found: {args.input_dir}")

    pdfs = sorted(args.input_dir.glob("*.pdf"))
    if args.limit and args.limit > 0:
        pdfs = pdfs[: args.limit]
    if not pdfs:
        sys.exit(f"no PDFs in {args.input_dir}")

    args.out_root.mkdir(parents=True, exist_ok=True)
    batch_log = args.out_root / "batch_manifest.json"
    results = []
    if args.resume and batch_log.is_file():
        try:
            results = json.loads(batch_log.read_text(encoding="utf-8")).get("results", [])
        except json.JSONDecodeError:
            results = []
    done_files = {r.get("file") for r in results if r.get("ok")}

    print(f"· {len(pdfs)} PDF(s) → {args.out_root}")
    print(f"· model={args.model if not args.no_llm else '(none)'}  ollama={args.ollama_url}")

    for i, pdf in enumerate(pdfs, 1):
        # Package dir named after stem (C2004A00601 etc.) until title parse improves slug.
        out_dir = args.out_root / pdf.stem
        slug_guess = pdf.stem.lower()
        if pdf.name in done_files and (out_dir / "manifest.json").is_file():
            print(f"[{i}/{len(pdfs)}] skip (done) {pdf.name}")
            continue

        cmd = [
            sys.executable,
            "-u",  # unbuffered child stdout so per-segment progress streams live, not in a burst
            str(LEGIS2CML),
            "--input",
            str(pdf),
            "--out-dir",
            str(out_dir),
            "--jurisdiction",
            args.jurisdiction,
            "--base-iri",
            f"{args.base_iri_prefix.rstrip('/')}/{pdf.stem.lower()}",
            "--ollama-url",
            args.ollama_url,
            "--model",
            args.model,
            "--max-segment-chars",
            str(args.max_segment_chars),
            "--max-segment-items",
            str(args.max_segment_items),
            "--segment-overlap-chars",
            str(args.segment_overlap_chars),
            "--ollama-timeout",
            str(args.ollama_timeout),
            "--max-retries",
            str(args.max_retries),
        ]
        if args.no_llm:
            cmd.append("--no-llm")
        if args.resume:
            cmd.append("--resume")
        if args.link:
            cmd.append("--link")
        if args.emit_ttl:
            cmd.append("--emit-ttl")
        if args.emit_q42:
            cmd.append("--emit-q42")
        if args.qualia_cli:
            cmd.extend(["--qualia-cli", str(args.qualia_cli)])
        cmd.extend(["--q42-shard-provisions", str(args.q42_shard_provisions)])

        print(f"[{i}/{len(pdfs)}] {pdf.name}")
        t0 = datetime.now(timezone.utc)
        try:
            rc, out_tail = run_child(cmd, prefix="    ", timeout=args.file_timeout)
        except KeyboardInterrupt:
            print(f"\n· interrupted on {pdf.name} — child stopped; per-segment checkpoints are "
                  f"saved. Rerun with --resume to continue.")
            break
        ok = rc == 0
        entry = {
            "file": pdf.name,
            "ok": ok,
            "returncode": rc,
            "out_dir": str(out_dir),
            "started": t0.isoformat(),
            "finished": datetime.now(timezone.utc).isoformat(),
            "output_tail": out_tail[-1200:],
        }
        # Prefer slug from package manifest when present
        man = out_dir / "manifest.json"
        if man.is_file():
            try:
                entry["slug"] = json.loads(man.read_text(encoding="utf-8")).get("slug")
            except json.JSONDecodeError:
                entry["slug"] = slug_guess
        else:
            entry["slug"] = slug_guess
        results = [result for result in results if result.get("file") != pdf.name]
        results.append(entry)
        batch_state = json.dumps(
                {
                    "generatedAt": datetime.now(timezone.utc).isoformat(),
                    "input_dir": str(args.input_dir),
                    "out_root": str(args.out_root),
                    "model": None if args.no_llm else args.model,
                    "segment_config": {"max_chars": args.max_segment_chars,
                                       "max_items": args.max_segment_items,
                                       "overlap_chars": args.segment_overlap_chars},
                    "count": len(results),
                    "ok": sum(1 for r in results if r.get("ok")),
                    "results": results,
                },
                indent=2,
            )
        batch_temp = batch_log.with_suffix(batch_log.suffix + ".tmp")
        batch_temp.write_text(batch_state, encoding="utf-8")
        batch_temp.replace(batch_log)
        if not ok:
            # rc=2 means some segments stayed pending (rerun --resume); output already streamed.
            print(f"  ! failed rc={rc} (see streamed output above)")
        else:
            print(f"  ok → {out_dir}")

    ok_n = sum(1 for r in results if r.get("ok"))
    print(f"· batch complete: {ok_n}/{len(results)} ok  log={batch_log}")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n· interrupted; progress checkpointed. Rerun with --resume.", file=sys.stderr)
        sys.exit(130)
