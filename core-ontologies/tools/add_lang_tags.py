#!/usr/bin/env python3
"""Surgically add the BCP-47 @en language tag to natural-language literals in the existing
values corpus (un-instruments + concepts) — content-preserving (does NOT regenerate, so
curation is not clobbered). Tags dc:title / values:originalText / skos:prefLabel literals
that are not already language/datatype-tagged. The generators now emit @en directly, so
this is a one-off retrofit of files generated before that.

Other languages (en-au, en-GB, fr, mother tongues, …) attach as ADDITIONAL @lang literals
on the same subject — this just flags the English source so the multilingual layer has a
base to build on.

Run: python3 core-ontologies/tools/add_lang_tags.py
"""
import re, glob

# Predicate + a full quoted literal (handling \" escapes), then the ;/. terminator. If the
# literal is already @lang / ^^datatype tagged, group 2 (the terminator) won't match → skip.
PAT = re.compile(
    r'^(\s*(?:dc:title|values:originalText|skos:prefLabel)\s+"(?:[^"\\]|\\.)*")(\s*[;.].*)$'
)

def tag_file(path):
    text = open(path, encoding="utf-8").read().replace("\r\n", "\n").replace("\r", "\n")
    out, changed = [], 0
    for ln in text.split("\n"):
        m = PAT.match(ln)
        if m:
            out.append(m.group(1) + "@en" + m.group(2))
            changed += 1
        else:
            out.append(ln)
    open(path, "w", encoding="utf-8", newline="\n").write("\n".join(out))
    return changed

def main():
    total = files = 0
    for pat in ("core-ontologies/un-instruments/*.n3", "core-ontologies/concepts/*.n3"):
        for f in sorted(glob.glob(pat)):
            c = tag_file(f)
            if c:
                files += 1
                total += c
    print(f"Tagged {total} literals @en across {files} files (content-preserving).")

if __name__ == "__main__":
    main()
