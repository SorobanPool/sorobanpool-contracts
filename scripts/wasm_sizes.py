#!/usr/bin/env python3
"""Compares contract wasm sizes against docs/wasm-size-budget.json.

Emits a markdown comparison table and fails if growth exceeds allowed margin.
Writes table to $GITHUB_STEP_SUMMARY when running under GitHub Actions.
"""
import glob
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BUDGET_PATH = os.path.join(ROOT, "docs", "wasm-size-budget.json")


def load_budget():
    if not os.path.exists(BUDGET_PATH):
        return {"max_growth_pct": 5.0, "budgets": {}}
    with open(BUDGET_PATH) as f:
        return json.load(f)


def find_wasms():
    patterns = [
        os.path.join(ROOT, "target", "wasm32v1-none", "release", "*.wasm"),
        os.path.join(ROOT, "target", "wasm32-unknown-unknown", "release", "*.wasm"),
    ]
    for p in patterns:
        files = glob.glob(p)
        if files:
            return sorted(files)
    return []


def main():
    budget_data = load_budget()
    max_growth = budget_data.get("max_growth_pct", 5.0)
    budgets = budget_data.get("budgets", {})

    wasms = find_wasms()
    if not wasms:
        print("No built wasm files found. Build contracts first with 'stellar contract build' or cargo.")
        return 0

    if "--update" in sys.argv:
        new_budgets = {}
        for w in wasms:
            name = os.path.basename(w)
            new_budgets[name] = os.path.getsize(w)
        budget_data["budgets"] = new_budgets
        with open(BUDGET_PATH, "w") as f:
            json.dump(budget_data, f, indent=2)
            f.write("\n")
        print(f"Updated {BUDGET_PATH} with current wasm sizes.")
        return 0

    rows = []
    failed = False

    rows.append("| Contract | Size (bytes) | Budget (bytes) | Growth | Status |")
    rows.append("| :--- | :---: | :---: | :---: | :---: |")

    for w in wasms:
        name = os.path.basename(w)
        size = os.path.getsize(w)
        budget = budgets.get(name)

        if budget is None:
            status = "NEW"
            growth_str = "N/A"
            rows.append(f"| `{name}` | {size:,} | N/A | {growth_str} | ⚠️ {status} |")
        else:
            pct_change = ((size - budget) / budget) * 100.0
            if pct_change > max_growth:
                failed = True
                status = f"FAILED (+{pct_change:.1f}% > +{max_growth:.1f}%)"
                rows.append(f"| `{name}` | {size:,} | {budget:,} | +{pct_change:.1f}% | ❌ {status} |")
            else:
                sign = "+" if pct_change > 0 else ""
                status = "PASS"
                rows.append(f"| `{name}` | {size:,} | {budget:,} | {sign}{pct_change:.1f}% | ✅ {status} |")

    table = "\n".join(rows)
    print("\n### Wasm Size Report\n")
    print(table)
    print()

    summary_file = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_file:
        with open(summary_file, "a") as f:
            f.write("\n### Wasm Size Report\n\n")
            f.write(table)
            f.write("\n\n")

    if failed:
        print(f"ERROR: One or more contracts grew beyond allowed margin (+{max_growth}%).")
        print("To update the size budget, run: python3 scripts/wasm_sizes.py --update")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
