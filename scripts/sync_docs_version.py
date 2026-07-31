#!/usr/bin/env python3
"""Sync hardcoded docs version strings with qualia-core-db Cargo.toml.

Keeps live product pages (benchmarks, demos, menu, package.json) on the current
engine semver. Historical progress-0.0.N.html archives are left alone.
"""
from __future__ import annotations

import re
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs"
CARGO = REPO / "crates" / "qualia-core-db" / "Cargo.toml"

# Never rewrite these (historical measurement dumps, changelogs, archives).
SKIP_NAME_GLOBS = {
    "comparative_benchmark_results*.json",
    "CHANGELOG.md",
    "RELEASE_NOTES_*.md",
    "CRYPTO_STATUS_*.md",
    "PROJECT_STATE.md",
    "release-targets.md",
    "progress-0.0.*.html",
}


def read_version() -> str:
    data = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    return data["package"]["version"]


def should_skip(path: Path) -> bool:
    name = path.name
    for pat in SKIP_NAME_GLOBS:
        if Path(name).match(pat):
            return True
    if "rustdoc" in path.parts:
        return True
    if path.suffix == ".q42":
        return True
    # WordNet VERSION tracks the GitHub Release that ships princeton.q42, not engine semver.
    if path == DOCS / "data" / "wordnet" / "VERSION":
        return True
    # Historical plan/handover notes under docs/plans keep their original branch labels.
    if "plans" in path.parts and path.suffix == ".md":
        return True
    if path.name.startswith("HANDOVER") and path.suffix == ".md":
        return True
    return False


def build_replacements(version: str) -> list[tuple[re.Pattern[str], str]]:
    """Map older 0.0.x product pins onto the current engine version.

    We only rewrite 0.0.N (N < current patch) product markers, not arbitrary
    decimals (e.g. Schema.org 30.0, Binaryen versions).
    """
    major_minor, _, patch_s = version.rpartition(".")
    if major_minor != "0.0":
        # Non-0.0.x scheme: only rewrite exact older known pins via package.json path.
        return []
    try:
        cur_patch = int(patch_s)
    except ValueError:
        return []

    reps: list[tuple[re.Pattern[str], str]] = []
    # Rewrite v0.0.N and bare 0.0.N for all N from 1 .. cur_patch-1
    for n in range(1, cur_patch):
        old = f"0.0.{n}"
        reps.append((re.compile(rf"\bv{re.escape(old)}\b"), f"v{version}"))
        reps.append((re.compile(rf"\b{re.escape(old)}-dev\b"), f"{version}-dev"))
        # Bare semver: not preceded/followed by a digit or '.'
        reps.append((re.compile(rf"(?<![\d.]){re.escape(old)}(?![\d.])"), version))
    return reps


def sync_file(path: Path, version: str, replacements: list[tuple[re.Pattern[str], str]]) -> bool:
    if should_skip(path):
        return False
    text = path.read_text(encoding="utf-8", errors="replace")
    original = text
    for pattern, repl in replacements:
        text = pattern.sub(repl, text)
    # package.json "version": "x.y.z"
    if path.name == "package.json":
        text = re.sub(
            r'("version"\s*:\s*")[\d.]+(")',
            rf"\g<1>{version}\2",
            text,
        )
    # engine_version fields in live benchmark JSON (not comparative dumps)
    if path.name == "llm_benchmark_results.json":
        text = re.sub(
            r'("engine_version"\s*:\s*")[\d.]+(")',
            rf"\g<1>{version}\2",
            text,
        )
    if text != original:
        path.write_text(text, encoding="utf-8", newline="\n")
        return True
    return False


def main() -> None:
    version = read_version()
    print(f"Canonical engine version: {version}")
    replacements = build_replacements(version)
    changed: list[str] = []
    for ext in (".html", ".js", ".md", ".json", ".yml"):
        for path in DOCS.rglob(f"*{ext}"):
            if sync_file(path, version, replacements):
                changed.append(str(path.relative_to(REPO)))
    # scripts that pin release tags for docs assets
    for script in (
        REPO / "scripts" / "fetch_anatomy_packs_release.sh",
        REPO / "docs" / "playground" / "anatomy.js",
    ):
        if script.is_file() and sync_file(script, version, replacements):
            changed.append(str(script.relative_to(REPO)))
    menu = DOCS / "menu.json"
    if menu.is_file():
        data = menu.read_text(encoding="utf-8")
        new = re.sub(r'"version"\s*:\s*"[\d.]+"', f'"version": "{version}"', data)
        if new != data:
            menu.write_text(new, encoding="utf-8", newline="\n")
            if "docs/menu.json" not in changed:
                changed.append("docs/menu.json")
    print(f"Updated {len(changed)} files")
    for p in sorted(set(changed))[:60]:
        print(f"  {p}")
    if len(changed) > 60:
        print(f"  … and {len(changed) - 60} more")


if __name__ == "__main__":
    main()
