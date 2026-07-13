#!/usr/bin/env python3
"""reprocess_library.py — the CML library-upgrade driver (see ../CML_UPGRADE.md).

Regenerates the GENERATED layer from SOURCE at the current SCHEMA_VERSION:
    concepts/*.n3  →  dist/q42/*.q42  →  docs/data/*.json  →  INDEX.md
Idempotent. Touches ONLY the machine-PROPOSED layer; human curation (overlays/**, the
generator KEEP set, cml:Attested / skos:exactMatch) is never rewritten.

    python core-ontologies/tools/reprocess_library.py            # full reprocess
    python core-ontologies/tools/reprocess_library.py --check     # report stale artifacts, write nothing (CI gate)

`--check` exits non-zero if any generated concept's cml:schemaVersion != SCHEMA_VERSION — so
"the logic changed but the library wasn't reprocessed" is caught.
"""
from __future__ import annotations
import glob
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent      # core-ontologies/
REPO = ROOT.parent                                  # repo root (scripts are repo-root-relative)
CONCEPTS = ROOT / "concepts"

# Hand-crafted / overlay concept files that are NOT machine-generated (never staleness-checked).
NON_GENERATED = {"duty-to-suppress-forced-labour", "iccpr-non-derogable-overlay"}


def schema_version() -> str:
    return (ROOT / "SCHEMA_VERSION").read_text(encoding="utf-8").strip()


def concept_version(path: Path) -> str | None:
    """The cml:schemaVersion stamped in a concept file, or None if unstamped."""
    txt = path.read_text(encoding="utf-8", errors="replace")
    m = re.search(r'cml:schemaVersion\s+"([^"]+)"', txt)
    return m.group(1) if m else None


def check() -> int:
    """Report generated concepts whose schemaVersion != current. Returns process exit code."""
    cur = schema_version()
    stale, unstamped = [], []
    for f in sorted(CONCEPTS.glob("*.n3")):
        if f.stem in NON_GENERATED:
            continue
        v = concept_version(f)
        if v is None:
            unstamped.append(f.name)
        elif v != cur:
            stale.append(f"{f.name} (v{v} ≠ v{cur})")
    print(f"CML_SCHEMA_VERSION = {cur}")
    print(f"Generated concepts: {len(list(CONCEPTS.glob('*.n3'))) - len(NON_GENERATED)}")
    if not stale and not unstamped:
        print("✓ library is up to date — no reprocessing needed.")
        return 0
    if unstamped:
        print(f"UNSTAMPED ({len(unstamped)}): {', '.join(unstamped[:8])}{' …' if len(unstamped) > 8 else ''}")
    if stale:
        print(f"STALE ({len(stale)}): {', '.join(stale[:8])}{' …' if len(stale) > 8 else ''}")
    print("→ run: python core-ontologies/tools/reprocess_library.py")
    return 1


def run(label: str, *args: str) -> None:
    print(f"\n=== {label} ===")
    p = subprocess.run([sys.executable, *args], cwd=REPO)
    if p.returncode != 0:
        sys.exit(f"step failed: {label} (exit {p.returncode})")


def reprocess() -> None:
    cur = schema_version()
    print(f"Reprocessing the CML library at SCHEMA_VERSION = {cur}")
    print("(machine-PROPOSED layer only; overlays/** + KEEP + attestations untouched)")
    t = "core-ontologies/tools"
    run("1/4 concepts  (generate_cml_concepts)", f"{t}/generate_cml_concepts.py")
    run("2/4 volumes   (build_q42)", f"{t}/build_q42.py")
    run("3/4 demo data (build_demo_data)", f"{t}/build_demo_data.py")
    if (ROOT / "tools" / "build_index.py").exists():
        run("4/4 index     (build_index)", f"{t}/build_index.py")
    print(f"\n✓ reprocess complete at v{cur}. Review the diff (generated layer only), run the "
          f"Rust gate (cargo test -p qualia-core-db --test validate_core_ontologies), commit "
          f"SCHEMA_VERSION + concepts/ + CML_VERSIONS.md together.")


def main() -> None:
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    if "--check" in sys.argv:
        sys.exit(check())
    reprocess()


if __name__ == "__main__":
    main()
