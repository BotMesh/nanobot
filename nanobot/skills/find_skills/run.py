"""Runtime helpers for the find-skills bundled skill."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Optional

from nanobot.utils.helpers import get_skills_path


def _scan_dir_for_skills(p: Path) -> list:
    if not p.exists() or not p.is_dir():
        return []
    return sorted([q.name for q in p.iterdir() if q.is_dir() and (q / "SKILL.md").exists()])


def list_skills(workspace: Path | None = None, query: Optional[str] = None) -> dict:
    """Return available skills grouped by 'system' and 'workspace'.

    If `query` is provided, skill names must contain the query (case-insensitive).
    """
    pkg_skills = Path(__file__).parent
    system = _scan_dir_for_skills(pkg_skills)

    ws_dir = get_skills_path(workspace)
    workspace_skills = _scan_dir_for_skills(ws_dir)

    if query:
        q = query.lower()
        system = [s for s in system if q in s.lower()]
        workspace_skills = [s for s in workspace_skills if q in s.lower()]

    return {"system": system, "workspace": workspace_skills}


def main(query: Optional[str] = None, json_out: bool = False):
    skills = list_skills(query=query)
    if json_out:
        print(json.dumps(skills, indent=2, ensure_ascii=False))
    else:
        print("System skills:")
        for s in skills.get("system", []):
            print(f"  - {s}")
        print("\nWorkspace skills:")
        for s in skills.get("workspace", []):
            print(f"  - {s}")


if __name__ == "__main__":
    main()
