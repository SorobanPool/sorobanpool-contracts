#!/usr/bin/env python3
"""
Checks markdown links across docs/ and README.md.
Internal broken links cause failure (exit code 1).
External links are checked with best-effort warnings and never fail the build.
"""
import re
import sys
import urllib.request
import urllib.error
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

LINK_RE = re.compile(r"(?<!\!)\[(?P<text>[^\]]+)\]\((?P<url>[^)]+)\)")

def check_external_url(url: str) -> bool:
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Mozilla/5.0 (Markdown-Link-Checker/1.0)"}
    )
    try:
        with urllib.request.urlopen(req, timeout=4) as response:
            return response.status < 400
    except Exception:
        return False

def main():
    docs_dir = ROOT / "docs"
    md_files = sorted(docs_dir.rglob("*.md"))
    readme = ROOT / "README.md"
    if readme.exists():
        md_files.append(readme)
    contributing = ROOT / "CONTRIBUTING.md"
    if contributing.exists():
        md_files.append(contributing)

    internal_errors = []
    external_warnings = []
    checked_external = {}

    total_links = 0
    for md_file in md_files:
        rel_md = md_file.relative_to(ROOT)
        content = md_file.read_text(encoding="utf-8")

        for m in LINK_RE.finditer(content):
            raw_url = m.group("url").strip()
            # Strip anchors e.g. foo.md#header
            url_target = raw_url.split("#")[0].strip()

            if not url_target:
                continue

            total_links += 1

            if url_target.startswith(("http://", "https://")):
                if url_target not in checked_external:
                    ok = check_external_url(url_target)
                    checked_external[url_target] = ok
                if not checked_external[url_target]:
                    external_warnings.append(f"{rel_md}: [Warning: flaky external URL] {url_target}")
            elif url_target.startswith("mailto:"):
                continue
            else:
                # Internal relative link
                target_path = (md_file.parent / url_target).resolve()
                if not target_path.exists():
                    internal_errors.append(
                        f"{rel_md}: Broken link '{raw_url}' -> file not found at {target_path}"
                    )

    print(f"Validated {total_links} links across {len(md_files)} markdown files.")

    if external_warnings:
        print(f"\nExternal URL Warnings ({len(external_warnings)}):")
        for w in external_warnings:
            print(f"  - {w}")

    if internal_errors:
        print(f"\nInternal Broken Link Errors ({len(internal_errors)}):", file=sys.stderr)
        for err in internal_errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)

    print("\nOK: All internal markdown links are valid.")

if __name__ == "__main__":
    main()
