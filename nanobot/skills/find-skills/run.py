"""Runtime helpers for the find-skills bundled skill.

Provides a small CLI to list system-bundled and workspace-installed skills.
"""

from __future__ import annotations

import json
from pathlib import Path

from nanobot.utils.helpers import get_skills_path


def list_skills(workspace: Path | None = None) -> dict:
    skills = {}
    pkg_skills = Path(__file__).parent
    # system bundled
    system = sorted(
        [p.name for p in pkg_skills.iterdir() if p.is_dir() and (p / "SKILL.md").exists()]
    )
    skills["system"] = system

    # workspace
    ws_dir = get_skills_path(workspace)
    if ws_dir.exists():
        workspace_skills = sorted(
            [p.name for p in ws_dir.iterdir() if p.is_dir() and (p / "SKILL.md").exists()]
        )
    else:
        workspace_skills = []
    skills["workspace"] = workspace_skills

    return skills


def main():
    skills = list_skills()
    print(json.dumps(skills, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
