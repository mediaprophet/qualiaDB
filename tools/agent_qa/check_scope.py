#!/usr/bin/env python3
"""Check only the files owned by the 0.0.28 programme.

The wider working tree may contain concurrent user/agent work. A scoped gate
prevents an agent from rewriting or taking ownership of unrelated files while
still enforcing deterministic formatting on this programme.
"""

from __future__ import annotations

import json
import pathlib
import subprocess


def main() -> int:
    tool_dir = pathlib.Path(__file__).resolve().parent
    root = tool_dir.parents[1]
    manifest = json.loads((tool_dir / "scope.json").read_text("utf-8"))
    paths = [root / relative for relative in manifest["rustfmt"]]
    missing = [str(path.relative_to(root)) for path in paths if not path.is_file()]
    if missing:
        print("Missing scoped files:")
        print("\n".join(missing))
        return 2
    command = ["rustfmt", "--edition", "2021", "--check", *map(str, paths)]
    completed = subprocess.run(command, cwd=root, check=False)
    if completed.returncode == 0:
        print(f"Scoped formatting passed for {len(paths)} Rust files.")
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
