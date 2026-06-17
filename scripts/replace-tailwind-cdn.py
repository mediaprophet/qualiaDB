#!/usr/bin/env python3
"""Replace Tailwind Play CDN script with built css/tailwind-built.css (per-page relative path)."""
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs"
CDN = '<script src="https://cdn.tailwindcss.com"></script>'
LAYOUT_LINK = '<link rel="stylesheet" href="{{ \'/css/tailwind-built.css\' | relative_url }}">'


def css_href(html_path: Path) -> str:
    rel = html_path.relative_to(DOCS)
    depth = len(rel.parts) - 1  # index.html -> 0, playground/x.html -> 1
    prefix = "../" * depth if depth else ""
    return f'{prefix}css/tailwind-built.css'


def main() -> None:
    changed = 0
    for path in DOCS.rglob("*.html"):
        if "init-draft-standards" in str(path):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if CDN not in text:
            continue
        if path == DOCS / "_layouts" / "qualia.html":
            new = text.replace(CDN, LAYOUT_LINK)
        else:
            href = css_href(path)
            new = text.replace(CDN, f'<link rel="stylesheet" href="{href}">')
        if new != text:
            path.write_text(new, encoding="utf-8", newline="\n")
            changed += 1
            print(path.relative_to(REPO))
    print(f"replaced Tailwind CDN on {changed} files")


if __name__ == "__main__":
    main()