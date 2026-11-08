#!/usr/bin/env python3
"""Verify public Markdown links to local repository files and folders."""
from pathlib import Path
from urllib.parse import unquote
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
HTML = re.compile(r'\b(?:src|href)\s*=\s*["\']([^"\']+)["\']', re.I)
IGNORE = ("https://", "http://", "mailto:", "data:", "#", "//")

def main() -> int:
    entries = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
    documents = [
        ROOT / entry.decode("utf-8")
        for entry in entries.split(b"\0") if entry and entry.lower().endswith(b".md")
    ]
    invalid = []
    checked = 0
    for doc in documents:
        text = doc.read_text(encoding="utf-8")
        # Inline code and code fences contain example strings, not Markdown links.
        text = re.sub(r"```.*?```", "", text, flags=re.S)
        links = LINK.findall(text) + HTML.findall(text)
        for link in links:
            raw = link.strip().split(" ", 1)[0].strip("<>")
            if not raw or raw.startswith(IGNORE):
                continue
            target_name = unquote(raw.split("#", 1)[0].split("?", 1)[0])
            if not target_name:
                continue
            target = (doc.parent / target_name).resolve()
            checked += 1
            if not target.is_relative_to(ROOT) or not target.exists():
                invalid.append(f"{doc.relative_to(ROOT)}: {raw}")
    if invalid:
        print("Broken local documentation links:")
        for item in invalid:
            print("  " + item)
        return 1
    print(f"Documentation links: PASS ({checked} local targets in {len(documents)} Markdown files)")
    return 0

if __name__ == "__main__":
    sys.exit(main())
