#!/usr/bin/env python3
"""Regenerate FEATURES.md from features.toml (the single source of truth).

Usage:  python3 tools/scripts/gen_docs.py
"""
import tomllib
from collections import Counter
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BADGE = {"live": "✅ LIVE", "building": "🚧 BUILDING", "planned": "📋 PLANNED"}
ORDER = {"live": 0, "building": 1, "planned": 2}


def main() -> None:
    data = tomllib.loads((ROOT / "features.toml").read_text())
    feats = sorted(data["feature"], key=lambda f: (ORDER[f["status"]], f["pillar"]))
    counts = Counter(f["status"] for f in feats)

    lines = [
        "# CodeIO Feature Status",
        "",
        "<!-- GENERATED FILE — do not edit. Edit features.toml and run tools/scripts/gen_docs.py -->",
        "",
        f"_Regenerated {date.today().isoformat()} — "
        f"{counts.get('live', 0)} live · {counts.get('building', 0)} building · "
        f"{counts.get('planned', 0)} planned_",
        "",
        "Legend: ✅ LIVE = working end-to-end · 🚧 BUILDING = code exists, not yet proven · 📋 PLANNED = theory/design only",
        "",
        "| Status | Feature | Pillar | Entry point | Description |",
        "|--------|---------|--------|-------------|-------------|",
    ]
    for f in feats:
        entry = f["entry"] if f["entry"] != "-" else "—"
        lines.append(
            f"| {BADGE[f['status']]} | **{f['name']}** | {f['pillar']} | `{entry}` | {f['desc']} |"
        )
    lines += [
        "",
        "See `VISION.md` for the pillars and `ROADMAP.md` for milestone tracking.",
        "",
    ]
    (ROOT / "FEATURES.md").write_text("\n".join(lines))
    print(f"FEATURES.md regenerated: {counts.get('live',0)} live, "
          f"{counts.get('building',0)} building, {counts.get('planned',0)} planned")


if __name__ == "__main__":
    main()
