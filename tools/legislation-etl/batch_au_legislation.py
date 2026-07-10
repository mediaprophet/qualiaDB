#!/usr/bin/env python3
"""
Batch AU federal legislation PDFs → CML/COF packages (ns.webcivics-style).

Default corpus:
  C:\\Users\\Admin\\Downloads\\20260630_AU-FED-LEGISLATION

Uses local Ollama (optional --no-llm for structure-only smoke). Page-level
resume via legis2cml progress files so multi-day runs survive restarts.

Examples:
  python batch_au_legislation.py --limit 2 --no-llm
  python batch_au_legislation.py --model llama3.2 --workers 1
  python batch_au_legislation.py --resume --ollama-url http://127.0.0.1:11434
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
LEGIS2CML = HERE / "legis2cml.py"
DEFAULT_IN = Path(r"C:\Users\Admin\Downloads\20260630_AU-FED-LEGISLATION")
DEFAULT_OUT = Path(r"C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation")


def main() -> None:
    ap = argparse.ArgumentParser(description="Batch AU legislation → CML/COF packages")
    ap.add_argument("--input-dir", type=Path, default=DEFAULT_IN)
    ap.add_argument("--out-root", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--model", default="llama3.2")
    ap.add_argument("--ollama-url", default="http://127.0.0.1:11434")
    ap.add_argument("--limit", type=int, default=0, help="process only first N PDFs (0 = all)")
    ap.add_argument("--no-llm", action="store_true")
    ap.add_argument("--resume", action="store_true", help="pass --resume to legis2cml")
    ap.add_argument("--link", action="store_true", help="do not copy PDFs into packages")
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
    done_slugs = {r.get("slug") for r in results if r.get("ok")}

    print(f"· {len(pdfs)} PDF(s) → {args.out_root}")
    print(f"· model={args.model if not args.no_llm else '(none)'}  ollama={args.ollama_url}")

    for i, pdf in enumerate(pdfs, 1):
        # Package dir named after stem (C2004A00601 etc.) until title parse improves slug.
        out_dir = args.out_root / pdf.stem
        slug_guess = pdf.stem.lower()
        if slug_guess in done_slugs and (out_dir / "manifest.json").is_file():
            print(f"[{i}/{len(pdfs)}] skip (done) {pdf.name}")
            continue

        cmd = [
            sys.executable,
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
        ]
        if args.no_llm:
            cmd.append("--no-llm")
        if args.resume:
            cmd.append("--resume")
        if args.link:
            cmd.append("--link")

        print(f"[{i}/{len(pdfs)}] {pdf.name}")
        t0 = datetime.now(timezone.utc)
        proc = subprocess.run(cmd, capture_output=True, text=True)
        ok = proc.returncode == 0
        entry = {
            "file": pdf.name,
            "ok": ok,
            "returncode": proc.returncode,
            "out_dir": str(out_dir),
            "started": t0.isoformat(),
            "finished": datetime.now(timezone.utc).isoformat(),
            "stderr_tail": (proc.stderr or "")[-800:],
            "stdout_tail": (proc.stdout or "")[-800:],
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
        results.append(entry)
        batch_log.write_text(
            json.dumps(
                {
                    "generatedAt": datetime.now(timezone.utc).isoformat(),
                    "input_dir": str(args.input_dir),
                    "out_root": str(args.out_root),
                    "model": None if args.no_llm else args.model,
                    "count": len(results),
                    "ok": sum(1 for r in results if r.get("ok")),
                    "results": results,
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        if not ok:
            print(f"  ! failed rc={proc.returncode}")
            if proc.stderr:
                print(proc.stderr[-400:])
        else:
            print(f"  ok → {out_dir}")

    ok_n = sum(1 for r in results if r.get("ok"))
    print(f"· batch complete: {ok_n}/{len(results)} ok  log={batch_log}")


if __name__ == "__main__":
    main()
