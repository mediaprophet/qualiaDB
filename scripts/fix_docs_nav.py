#!/usr/bin/env python3
"""Fix dynamic-nav containers and menu-loader script paths in docs HTML."""
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs"

NAV_PAT = re.compile(r'(<div\s+id="dynamic-nav"\s+)class="[^"]*"')
NAV_REPL = r'\1class="relative flex items-center text-sm"'


def main():
    fixed_nav = 0
    for path in DOCS.rglob("*.html"):
        if "init-draft-standards" in str(path):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        new, n = NAV_PAT.subn(NAV_REPL, text)
        if n:
            path.write_text(new, encoding="utf-8", newline="\n")
            fixed_nav += 1

    for path in (DOCS / "playground").glob("*.html"):
        text = path.read_text(encoding="utf-8", errors="replace")
        if 'src="js/menu-loader.js"' in text:
            text = text.replace('src="js/menu-loader.js"', 'src="../js/menu-loader.js"')
            path.write_text(text, encoding="utf-8", newline="\n")
            print(f"script: playground/{path.name}")

    llm = DOCS / "llmdemo" / "index.html"
    if llm.is_file():
        text = llm.read_text(encoding="utf-8", errors="replace")
        if 'src="js/menu-loader.js"' in text:
            llm.write_text(
                text.replace('src="js/menu-loader.js"', 'src="../js/menu-loader.js"'),
                encoding="utf-8",
                newline="\n",
            )
            print("script: llmdemo/index.html")

    print(f"fixed dynamic-nav on {fixed_nav} pages")


if __name__ == "__main__":
    main()