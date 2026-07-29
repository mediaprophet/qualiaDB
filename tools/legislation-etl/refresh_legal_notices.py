#!/usr/bin/env python3
"""Refresh the legal-information surface of generated AU legislation HTML files.

This is intentionally a presentation-only migration: it does not reparse PDFs,
change semantic data, or invoke the language model.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

import legis2cml


DEFAULT_ROOT = Path(
    r"C:\Projects\webcivics\ns\ns\public\institutions\au-fed-legislation"
)


def refresh_file(path: Path) -> bool:
    register_id = path.parent.name
    original = path.read_text(encoding="utf-8")
    updated = re.sub(
        r"<style>.*?</style>",
        lambda _match: f"<style>{legis2cml.CSS}</style>",
        original,
        count=1,
        flags=re.DOTALL,
    )

    rights_metadata = legis2cml.rights_metadata_html()
    if "<!-- rights-metadata:start -->" in updated:
        updated = re.sub(
            r"<!-- rights-metadata:start -->.*?<!-- rights-metadata:end -->",
            lambda _match: rights_metadata,
            updated,
            count=1,
            flags=re.DOTALL,
        )
    else:
        updated = updated.replace("</head>", f"{rights_metadata}\n</head>", 1)

    notice = legis2cml.legal_notice_html(register_id)
    if '<aside class="legal-notice"' in updated:
        updated = re.sub(
            r'<aside class="legal-notice".*?</aside>',
            lambda _match: notice,
            updated,
            count=1,
            flags=re.DOTALL,
        )
    else:
        updated = updated.replace("</header>", f"</header>\n{notice}", 1)

    footer = legis2cml.legal_footer_html(register_id)
    if '<footer class="legal-footer"' in updated:
        updated = re.sub(
            r'<footer class="legal-footer">.*?</footer>',
            lambda _match: footer,
            updated,
            count=1,
            flags=re.DOTALL,
        )
    else:
        updated = updated.replace(
            '<script type="application/ld+json">',
            f'{footer}\n<script type="application/ld+json">',
            1,
        )

    if updated == original:
        return False
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(updated, encoding="utf-8")
    temporary.replace(path)
    return True


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Refresh notices in generated AU legislation CML HTML files"
    )
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    if not args.root.is_dir():
        raise SystemExit(f"legislation root not found: {args.root}")

    files = sorted(args.root.glob("*/*.cml.html"))
    changed = sum(refresh_file(path) for path in files)
    print(f"refreshed {changed}/{len(files)} legislation page(s)")


if __name__ == "__main__":
    main()
