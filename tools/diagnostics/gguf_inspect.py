#!/usr/bin/env python3
"""CLI for bounded GGUF structural inspection."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from gguf_reader import GgufError, inspect_gguf


def sha256_file(path: Path, chunk_bytes: int = 1024 * 1024) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(chunk_bytes), b""):
            digest.update(block)
    return digest.hexdigest()


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("input", type=Path, help="GGUF model to inspect")
    result.add_argument("--json", action="store_true", help="write a JSON report to stdout")
    result.add_argument("--sha256", action="store_true", help="stream a SHA-256 digest")
    result.add_argument("--max-metadata-mib", type=int, default=64)
    result.add_argument("--max-string-kib", type=int, default=1024)
    result.add_argument("--max-entries", type=int, default=1_000_000)
    result.add_argument("--max-tensors", type=int, default=50)
    result.add_argument("--tensor-prefix", help="only report tensor names with this prefix")
    return result


def main() -> int:
    args = parser().parse_args()
    if not args.input.is_file():
        print(f"error: not a file: {args.input}", file=sys.stderr)
        return 2
    if min(args.max_metadata_mib, args.max_string_kib, args.max_entries, args.max_tensors) < 1:
        print("error: all limits must be positive", file=sys.stderr)
        return 2
    try:
        report = inspect_gguf(
            args.input,
            max_metadata_bytes=args.max_metadata_mib * 1024 * 1024,
            max_string_bytes=args.max_string_kib * 1024,
            max_entries=args.max_entries,
            max_tensors=args.max_tensors,
            tensor_prefix=args.tensor_prefix,
        ).as_dict()
        if args.sha256:
            report["sha256"] = sha256_file(args.input)
    except (GgufError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0
    print(f"GGUF v{report['version']} — {report['file_bytes']:,} bytes")
    print(f"KV entries: {report['key_value_count']:,}; tensors: {report['tensor_count']:,}")
    print(f"alignment: {report['alignment']}; tensor data begins: {report['tensor_data_offset']:,}")
    for key, value in report["metadata"].items():
        print(f"{key}: {value}")
    for tensor in report["tensors"]:
        print(f"tensor {tensor['name']}: dims={tensor['dimensions']} type={tensor['ggml_type']} offset={tensor['data_offset']}")
    if report["tensors_truncated"]:
        print("tensor listing truncated; increase --max-tensors or use --tensor-prefix")
    if "sha256" in report:
        print(f"sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
