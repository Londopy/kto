#!/usr/bin/env python3
"""Fill the release-notes template with the version and the SHA-256 sums."""
import pathlib
import sys

ver = sys.argv[1]
tpl = pathlib.Path("docs/release_body.md").read_text(encoding="utf-8")

sums_file = pathlib.Path("artifacts") / f"kto-{ver}-SHA256SUMS.txt"
sums = sums_file.read_text(encoding="utf-8").strip() if sums_file.exists() else "(checksums unavailable)"

body = tpl.replace("__VERSION__", ver).replace("__CHECKSUMS__", "```\n" + sums + "\n```")
pathlib.Path("RELEASE_BODY.md").write_text(body, encoding="utf-8")
print(body)
