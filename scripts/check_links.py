#!/usr/bin/env python3
"""Verify every relative markdown link in docs/, README.md, ROADMAP.md resolves.

Run from the repo root: python3 scripts/check_links.py
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)#]+)(?:#[^)]*)?\)")

files = []
for base in ("docs",):
    for dirpath, _, names in os.walk(os.path.join(ROOT, base)):
        for n in names:
            if n.endswith(".md"):
                files.append(os.path.join(dirpath, n))
files += [os.path.join(ROOT, f) for f in ("README.md", "ROADMAP.md")]

missing = 0
for path in files:
    with open(path, encoding="utf-8") as f:
        text = f.read()
    for m in LINK_RE.finditer(text):
        target = m.group(1).strip()
        if not target or target.startswith(("http://", "https://", "#")):
            continue
        resolved = os.path.normpath(os.path.join(os.path.dirname(path), target))
        if not os.path.exists(resolved):
            print(f"broken link: {os.path.relpath(path, ROOT)} -> {target}")
            missing += 1

if missing:
    print(f"{missing} broken links")
    sys.exit(1)
print(f"{len(files)} files, all links resolve")