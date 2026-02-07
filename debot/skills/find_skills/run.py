"""Runtime helpers for the find-skills bundled skill."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Optional

from debot.utils.helpers import get_skills_path


def _list_dir_skills(dir_path: Path) -> List[str]:
    skills: List[str] = []
    if not dir_path.exists() or not dir_path.is_dir():
        return skills

    for p in sorted(dir_path.iterdir()):
        if p.name in ("find_skills", "__pycache__"):
            continue
        if p.is_dir():
            if (p / "SKILL.md").exists() or any(p.glob("*.md")):
                skills.append(p.name)
    return skills


def list_skills(workspace: str | Path, query: str | None = None) -> Dict[str, List[str]]:
    """Return available system and workspace skills.

    Args:
        workspace: workspace path (may be a Path or string)
        query: optional substring filter (case-insensitive)

    Returns:
        Dict with keys `system` and `workspace` mapping to lists of skill names.
    """
    ws = Path(workspace).expanduser()

    # System skills live next to this runtime in debot/skills
    system_dir = Path(__file__).resolve().parent.parent

    # Debug helper: if DEBOT_DEBUG env var set, print resolution info
    if os.environ.get("DEBOT_DEBUG"):
        print(f"DEBUG: system_dir={system_dir}", file=sys.stderr)
        print(f"DEBUG: entries={[p.name for p in sorted(system_dir.iterdir())]}", file=sys.stderr)

    system = _list_dir_skills(system_dir)

    # Workspace skills live under workspace/skills
    workspace_skills_dir = ws / "skills"
    workspace_list = _list_dir_skills(workspace_skills_dir)

    def _filter(lst: List[str]) -> List[str]:
        if not query:
            return lst
        q = query.lower()
        return [s for s in lst if q in s.lower()]

    return {"system": _filter(system), "workspace": _filter(workspace_list)}


def main():
    ws = Path("~/.debot/workspace").expanduser()
    skills = list_skills(ws)
    print(json.dumps(skills, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
