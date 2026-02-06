"""Memory system for persistent agent memory."""

import hashlib
import json
import math
import uuid
from datetime import datetime
from pathlib import Path
from typing import Dict, List

from nanobot.utils.helpers import ensure_dir, today_date


class MemoryStore:
    """
    Memory system for the agent.

    Supports daily notes (memory/YYYY-MM-DD.md) and long-term memory (MEMORY.md).
    """

    def __init__(self, workspace: Path):
        self.workspace = workspace
        self.memory_dir = ensure_dir(workspace / "memory")
        self.memory_file = self.memory_dir / "MEMORY.md"

    def get_today_file(self) -> Path:
        """Get path to today's memory file."""
        return self.memory_dir / f"{today_date()}.md"

    def read_today(self) -> str:
        """Read today's memory notes."""
        today_file = self.get_today_file()
        if today_file.exists():
            return today_file.read_text(encoding="utf-8")
        return ""

    def append_today(self, content: str) -> None:
        """Append content to today's memory notes."""
        today_file = self.get_today_file()

        if today_file.exists():
            existing = today_file.read_text(encoding="utf-8")
            content = existing + "\n" + content
        else:
            # Add header for new day
            header = f"# {today_date()}\n\n"
            content = header + content

        today_file.write_text(content, encoding="utf-8")

    def read_long_term(self) -> str:
        """Read long-term memory (MEMORY.md)."""
        if self.memory_file.exists():
            return self.memory_file.read_text(encoding="utf-8")
        return ""

    def write_long_term(self, content: str) -> None:
        """Write to long-term memory (MEMORY.md)."""
        self.memory_file.write_text(content, encoding="utf-8")

    def get_recent_memories(self, days: int = 7) -> str:
        """
        Get memories from the last N days.

        Args:
            days: Number of days to look back.

        Returns:
            Combined memory content.
        """
        from datetime import timedelta

        memories = []
        today = datetime.now().date()

        for i in range(days):
            date = today - timedelta(days=i)
            date_str = date.strftime("%Y-%m-%d")
            file_path = self.memory_dir / f"{date_str}.md"

            if file_path.exists():
                content = file_path.read_text(encoding="utf-8")
                memories.append(content)

        return "\n\n---\n\n".join(memories)

    def list_memory_files(self) -> list[Path]:
        """List all memory files sorted by date (newest first)."""
        if not self.memory_dir.exists():
            return []

        files = list(self.memory_dir.glob("????-??-??.md"))
        return sorted(files, reverse=True)

    def get_memory_context(self) -> str:
        """
        Get memory context for the agent.

        Returns:
            Formatted memory context including long-term and recent memories.
        """
        parts = []

        # Long-term memory
        long_term = self.read_long_term()
        if long_term:
            parts.append("## Long-term Memory\n" + long_term)

        # Today's notes
        today = self.read_today()
        if today:
            parts.append("## Today's Notes\n" + today)

        return "\n\n".join(parts) if parts else ""

    def build_index(self) -> int:
        """Build a simple local index for all markdown memory files.

        Returns the number of indexed chunks written to `.index.json`.
        """
        entries: List[Dict] = []

        if not self.memory_dir.exists():
            return 0

        for entry in self.memory_dir.iterdir():
            path = entry
            if not path.is_file() or not path.name.endswith(".md"):
                continue
            text = path.read_text(encoding="utf-8")
            chunk_size = 800
            overlap = 100
            start = 0
            length = len(text)
            while start < length:
                end = min(start + chunk_size, length)
                chunk = text[start:end]
                vec = _embed_local(chunk)
                entry_obj = {
                    "id": str(uuid.uuid4()),
                    "path": str(path.relative_to(self.workspace)),
                    "start_line": 0,
                    "end_line": 0,
                    "text": chunk,
                    "vector": vec,
                }
                entries.append(entry_obj)
                if end == length:
                    break
                start = max(0, end - overlap)

        index_path = self.memory_dir / ".index.json"
        index_path.write_text(json.dumps(entries, indent=2), encoding="utf-8")
        return len(entries)

    def search(self, query: str, max_results: int = 5, min_score: float = 0.0) -> List[Dict]:
        """Search the local index for semantically similar chunks.

        Returns a list of dicts: {path, snippet, score}.
        """
        index_path = self.memory_dir / ".index.json"
        if not index_path.exists():
            return []

        try:
            entries = json.loads(index_path.read_text(encoding="utf-8"))
        except Exception:
            return []

        qvec = _embed_local(query)
        scored = []
        for e in entries:
            vec = e.get("vector", [])
            score = _cosine_similarity(qvec, vec)
            if score >= min_score:
                scored.append((score, e))

        scored.sort(key=lambda x: x[0], reverse=True)
        results = []
        for score, e in scored[:max_results]:
            results.append(
                {"path": e.get("path", ""), "snippet": e.get("text", ""), "score": score}
            )
        return results


def _embed_local(text: str) -> List[float]:
    h = hashlib.sha256(text.encode("utf-8")).digest()
    dims = 64
    vec = [(b / 127.5) - 1.0 for i, b in enumerate(h * (dims // len(h) + 1))[:dims]]
    norm = math.sqrt(sum(x * x for x in vec))
    if norm < 1e-6:
        norm = 1e-6
    return [x / norm for x in vec]


def _cosine_similarity(a: List[float], b: List[float]) -> float:
    if not a or not b or len(a) != len(b):
        return 0.0
    return sum(x * y for x, y in zip(a, b))
