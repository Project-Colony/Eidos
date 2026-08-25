#!/usr/bin/env python3
"""Every markdown link and anchor in the docs resolves.

Split from i18n-check.sh because it needs a real slug function: an anchor is
GitHub's slug of a heading, and a translated page's headings are - obviously -
not the English ones. The failure this exists to catch is quiet and specific:
a translation that correctly rewrote `usage.md` to `usage.de.md` and correctly
left `#why-passthrough-is-off-by-default` alone, producing a link that lands on
the right page at the wrong place, or nowhere at all. Twenty-nine of those
shipped in one batch before this existed.
"""
import glob
import os
import re
import sys

ANCHOR = re.compile(r"\]\(([^)\s#]*\.md)(?:#([^)\s]+))?\)")


def slug(heading: str) -> str:
    s = heading.strip().lower()
    s = re.sub(r"`([^`]*)`", r"\1", s)
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
    s = re.sub(r"[*_]", "", s)
    s = "".join(c for c in s if c.isalnum() or c in " -_")
    return s.strip().replace(" ", "-")


def headings(path: str) -> set:
    with open(path, encoding="utf-8") as fh:
        return {slug(l.lstrip("#").strip()) for l in fh if l.startswith("#")}


def main() -> int:
    bad = 0
    files = sorted(f for f in glob.glob("**/*.md", recursive=True) if not f.startswith("target/"))
    for f in files:
        d = os.path.dirname(f)
        with open(f, encoding="utf-8") as fh:
            text = fh.read()
        for m in ANCHOR.finditer(text):
            target, anchor = m.group(1), m.group(2)
            if target.startswith("http"):
                continue
            p = os.path.normpath(os.path.join(d, target))
            if not os.path.exists(p):
                print(f"MISSING FILE   {f}\n               -> {target}")
                bad += 1
                continue
            if anchor and anchor not in headings(p):
                print(f"MISSING ANCHOR {f}\n               -> {target}#{anchor}")
                bad += 1
    print(f"\n{len(files)} markdown file(s) checked", end="")
    print(f", {bad} broken link(s)" if bad else ", every link resolves")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
