#!/usr/bin/env python3
"""Writes artifacts/errors.json (from the Error enums) and artifacts/events.json (from docs/events.md).
Consumed by the frontend error map and the backend indexer fixtures."""
import glob, json, os, re

root = os.path.join(os.path.dirname(__file__), "..")
os.makedirs(os.path.join(root, "artifacts"), exist_ok=True)

errors = []
for f in sorted(glob.glob(os.path.join(root, "contracts/*/src/lib.rs"))):
    contract = f.split("/")[-3]
    m = re.search(r"pub enum Error \{(.*?)\n\}", open(f).read(), re.S)
    if not m:
        continue
    for line in m.group(1).splitlines():
        mm = re.match(r"\s*(\w+)\s*=\s*(\d+),", line)
        if mm:
            errors.append({"code": int(mm.group(2)), "contract": contract, "name": mm.group(1)})
errors.sort(key=lambda e: e["code"])
json.dump(errors, open(os.path.join(root, "artifacts/errors.json"), "w"), indent=2)

events = []
for line in open(os.path.join(root, "docs/events.md")):
    cells = [c.strip().strip("`") for c in line.strip().strip("|").split("|")]
    if len(cells) == 4 and cells[0] not in ("Contract", "---") and not cells[0].startswith("-"):
        for sym in re.split(r"`? / `?", cells[1]):
            events.append({"contract": cells[0], "event": sym.strip("` "), "key": cells[2].replace("`", ""), "data": cells[3].replace("`", "")})
json.dump(events, open(os.path.join(root, "artifacts/events.json"), "w"), indent=2)
print(f"{len(errors)} errors, {len(events)} events")
