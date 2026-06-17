#!/usr/bin/env python3
"""Sync hardcoded docs version strings with qualia-core-db Cargo.toml."""
from __future__ import annotations

import re
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs"
CARGO = REPO / "crates" / "qualia-core-db" / "Cargo.toml"

SKIP_GLOBS = {
    "comparative_benchmark_results*.json",
    "CHANGELOG.md",
    "RELEASE_NOTES_*.md",
    "CRYPTO_STATUS_*.md",
    "PROJECT_STATE.md",
    "release-targets.md",
}

REPLACEMENTS = [
    (re.compile(r"\bv0\.0\.15\b"), "v0.0.17"),
    (re.compile(r"\bv0\.0\.16\b"), "v0.0.17"),
    (re.compile(r"\b0\.0\.15-dev\b"), "0.0.17-dev"),
    (re.compile(r"\b0\.0\.16-dev\b"), "0.0.17-dev"),
    (re.compile(r"(?<![\d.])0\.0\.15(?![\d])"), "0.0.17"),
    (re.compile(r"(?<![\d.])0\.0\.16(?![\d])"), "0.0.17"),
]


def read_version() -> str:
    data = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    return data["package"]["version"]


def should_skip(path: Path) -> bool:
    name = path.name
    for pat in SKIP_GLOBS:
        if Path(name).match(pat):
            return True
    if "rustdoc" in path.parts:
        return True
    if path.suffix == ".q42":
        return True
    return False


def sync_file(path: Path, version: str) -> bool:
    if should_skip(path):
        return False
    text = path.read_text(encoding="utf-8", errors="replace")
    original = text
    for pattern, repl in REPLACEMENTS:
        text = pattern.sub(repl, text)
    # package.json "version": "x.y.z"
    if path.name == "package.json":
        text = re.sub(
            r'("version"\s*:\s*")[\d.]+(")',
            rf'\g<1>{version}\2',
            text,
        )
    if text != original:
        path.write_text(text, encoding="utf-8", newline="\n")
        return True
    return False


def main() -> None:
    version = read_version()
    print(f"Canonical engine version: {version}")
    changed: list[str] = []
    for ext in (".html", ".js", ".md", ".json", ".yml"):
        for path in DOCS.rglob(f"*{ext}"):
            if sync_file(path, version):
                changed.append(str(path.relative_to(REPO)))
    # menu.json lives under docs
    menu = DOCS / "menu.json"
    if menu.is_file():
        data = menu.read_text(encoding="utf-8")
        new = re.sub(r'"version"\s*:\s*"[\d.]+"', f'"version": "{version}"', data)
        if new != data:
            menu.write_text(new, encoding="utf-8", newline="\n")
            changed.append("docs/menu.json")
    print(f"Updated {len(changed)} files")
    for p in sorted(changed)[:40]:
        print(f"  {p}")
    if len(changed) > 40:
        print(f"  … and {len(changed) - 40} more")


if __name__ == "__main__":
    main()