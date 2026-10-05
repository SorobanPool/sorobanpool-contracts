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

def emitted_from_source():
    """(contract_symbol, event_symbol) pairs the contracts really emit, parsed from emit(env, C, <event>, ...) calls."""
    out = set()
    for f in sorted(glob.glob(os.path.join(root, "contracts/*/src/lib.rs"))):
        src = open(f).read()
        m = re.search(r'const C: Symbol = symbol_short!\("(\w+)"\)', src)
        if not m:
            continue
        contract = m.group(1)
        for call in re.finditer(r"emit\(\s*&?env,\s*C,\s*([^,]+),", src):
            arg = call.group(1)
            names = re.findall(r'symbol_short!\("(\w+)"\)', arg)
            if not names and re.fullmatch(r"\w+", arg.strip()):
                # a variable, e.g. `let ev = if .. { symbol_short!("a") } else { symbol_short!("b") }`
                decl = re.search(r"let\s+%s\s*=([^;]+);" % re.escape(arg.strip()), src)
                names = re.findall(r'symbol_short!\("(\w+)"\)', decl.group(1)) if decl else []
            if not names:
                raise SystemExit(f"cannot determine event name in {f}: emit(..., {arg.strip()}, ...)")
            for n in names:
                out.add((contract, n))
    return out

documented = {(e["contract"], e["event"]) for e in events}
emitted = emitted_from_source()
problems = []
for c, e in sorted(emitted - documented):
    problems.append(f"emitted but not in docs/events.md: {c}.{e}")
for c, e in sorted(documented - emitted):
    problems.append(f"in docs/events.md but never emitted: {c}.{e}")
if problems:
    raise SystemExit("docs/events.md is out of sync with the contracts:\n  " + "\n  ".join(problems))

json.dump(events, open(os.path.join(root, "artifacts/events.json"), "w"), indent=2)
print(f"{len(errors)} errors, {len(events)} events (docs match the contracts)")
