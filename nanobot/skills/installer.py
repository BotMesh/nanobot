"""Installer utilities for Skills (supports direct URLs and system-bundled skills)."""

from __future__ import annotations

import shutil
import tempfile
import zipfile
from pathlib import Path
from typing import Optional
from urllib.request import Request, urlopen

from nanobot.utils.helpers import get_skills_path


def _download_to_temp(url: str) -> Path:
    req = Request(url, headers={"User-Agent": "nanobot/installer"})
    with urlopen(req) as resp:
        if resp.status != 200:
            raise RuntimeError(f"Failed to download: {url} (status={resp.status})")
        data = resp.read()

    fd, tmp = tempfile.mkstemp(suffix=".skill")
    p = Path(tmp)
    with open(p, "wb") as f:
        f.write(data)
    return p


def _extract_skill(zip_path: Path, target_dir: Path) -> None:
    # Extract zip (.skill) into target_dir/<skill-name>
    with zipfile.ZipFile(zip_path, "r") as z:
        # Try to determine skill name from SKILL.md frontmatter
        names = [n for n in z.namelist() if n.endswith("SKILL.md")]
        if names:
            # use first SKILL.md path's parent as skill dir
            base = Path(names[0]).parent
            dest = target_dir / base.name
        else:
            dest = target_dir / zip_path.stem

        if dest.exists():
            shutil.rmtree(dest)
        dest.mkdir(parents=True, exist_ok=True)
        z.extractall(path=dest)


def install_from_url(url: str, workspace: Optional[Path] = None) -> Path:
    """Download a .skill (zip) from `url` and install into workspace skills dir.

    Returns the installed skill path.
    """
    skills_dir = get_skills_path(workspace)
    tmp = _download_to_temp(url)
    try:
        _extract_skill(tmp, skills_dir)
        # Determine installed folder
        with zipfile.ZipFile(tmp, "r") as z:
            names = [n for n in z.namelist() if n.endswith("SKILL.md")]
            if names:
                installed = skills_dir / Path(names[0]).parent.name
            else:
                installed = skills_dir / tmp.stem
        return installed
    finally:
        try:
            Path(tmp).unlink()
        except Exception:
            pass


def install_from_system(name: str, workspace: Optional[Path] = None) -> Path:
    """Install a system-bundled skill from the package `nanobot/skills/<name>`.

    Copies the skill directory into the user's workspace skills directory.
    """
    skills_dir = get_skills_path(workspace)
    pkg_skills = Path(__file__).parent
    # allow both hyphenated skill names and underscore package dirs
    src = pkg_skills / name
    if not src.exists():
        alt = name.replace("-", "_")
        src = pkg_skills / alt
    if not src.exists() or not src.is_dir():
        # list available system skills
        available = sorted(
            [p.name for p in pkg_skills.iterdir() if p.is_dir() and (p / "SKILL.md").exists()]
        )
        raise RuntimeError(f"System skill '{name}' not found. Available: {', '.join(available)}")

    dest = skills_dir / name
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(src, dest)
    return dest
