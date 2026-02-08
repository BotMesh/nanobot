"""Session management for conversation history."""

import json
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any

from loguru import logger

from debot.utils.helpers import ensure_dir, safe_filename


@dataclass
class Session:
    """
    A conversation session.

    Stores messages in JSONL format for easy reading and persistence.
    """

    key: str  # channel:chat_id
    messages: list[dict[str, Any]] = field(default_factory=list)
    created_at: datetime = field(default_factory=datetime.now)
    updated_at: datetime = field(default_factory=datetime.now)
    metadata: dict[str, Any] = field(default_factory=dict)

    def add_message(self, role: str, content: str, **kwargs: Any) -> None:
        """Add a message to the session."""
        msg = {"role": role, "content": content, "timestamp": datetime.now().isoformat(), **kwargs}
        self.messages.append(msg)
        self.updated_at = datetime.now()

    def compact(self, keep_last: int = 50, instruction: str | None = None) -> int:
        """Compact older messages into a single summary entry.

        This summarizes all messages older than the last `keep_last` messages
        into one compaction entry stored as the first message. The method
        returns the number of messages that were compacted.
        """
        if keep_last < 0:
            keep_last = 0

        total = len(self.messages)
        if total <= keep_last:
            return 0

        # Messages to compact (older ones) and the recent window to keep
        old = self.messages[: max(0, total - keep_last)]
        recent = self.messages[max(0, total - keep_last) :]

        # Build a compact summary. This is intentionally simple and deterministic
        # so it can run offline; consumers can replace this with an LLM-based
        # summary later if desired.
        parts: list[str] = []
        if instruction:
            parts.append(f"Compaction instructions: {instruction}")

        parts.append(f"Compacted {len(old)} messages from session {self.key}.")
        # include brief excerpts from older messages
        for m in old:
            role = m.get("role", "unknown")
            content = (m.get("content") or "").replace("\n", " ")
            excerpt = content[:200]
            parts.append(f"- {role}: {excerpt}")

        summary = "\n".join(parts)

        compaction_entry = {
            "_type": "compaction",
            "role": "system",
            "content": f"🧹 Auto-compaction summary:\n\n{summary}",
            "timestamp": datetime.now().isoformat(),
            "compacted_count": len(old),
        }

        # Replace messages with a single compaction entry followed by the recent window
        self.messages = [compaction_entry] + recent
        self.updated_at = datetime.now()

        # Update metadata with compaction telemetry
        if "compactions" not in self.metadata:
            self.metadata["compactions"] = {
                "total": 0,
                "count": 0,
                "last_at": None,
                "messages_compacted": 0,
            }
        self.metadata["compactions"]["total"] += 1
        self.metadata["compactions"]["count"] += len(old)
        self.metadata["compactions"]["last_at"] = self.updated_at.isoformat()
        self.metadata["compactions"]["messages_compacted"] += len(old)

        return len(old)

    def get_history(self, max_messages: int = 50) -> list[dict[str, Any]]:
        """
        Get message history for LLM context.

        Args:
            max_messages: Maximum messages to return.

        Returns:
            List of messages in LLM format.
        """
        # Get recent messages
        recent = self.messages[-max_messages:] if len(self.messages) > max_messages else self.messages

        # Convert to LLM format (just role and content)
        return [{"role": m["role"], "content": m["content"]} for m in recent]

    def clear(self) -> None:
        """Clear all messages in the session."""
        self.messages = []
        self.updated_at = datetime.now()


class SessionManager:
    """
    Manages conversation sessions.

    Sessions are stored as JSONL files in the sessions directory.
    """

    def __init__(self, workspace: Path):
        self.workspace = workspace
        self.sessions_dir = ensure_dir(Path.home() / ".debot" / "sessions")
        self._cache: dict[str, Session] = {}

    def _get_session_path(self, key: str) -> Path:
        """Get the file path for a session."""
        safe_key = safe_filename(key.replace(":", "_"))
        return self.sessions_dir / f"{safe_key}.jsonl"

    def get_or_create(self, key: str) -> Session:
        """
        Get an existing session or create a new one.

        Args:
            key: Session key (usually channel:chat_id).

        Returns:
            The session.
        """
        # Check cache
        if key in self._cache:
            return self._cache[key]

        # Try to load from disk
        session = self._load(key)
        if session is None:
            session = Session(key=key)

        self._cache[key] = session
        return session

    def _load(self, key: str) -> Session | None:
        """Load a session from disk."""
        path = self._get_session_path(key)

        if not path.exists():
            return None

        try:
            messages = []
            metadata = {}
            created_at = None

            with open(path) as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue

                    data = json.loads(line)

                    if data.get("_type") == "metadata":
                        metadata = data.get("metadata", {})
                        created_at = datetime.fromisoformat(data["created_at"]) if data.get("created_at") else None
                    else:
                        messages.append(data)

            return Session(
                key=key,
                messages=messages,
                created_at=created_at or datetime.now(),
                metadata=metadata,
            )
        except Exception as e:
            logger.warning(f"Failed to load session {key}: {e}")
            return None

    def save(self, session: Session) -> None:
        """Save a session to disk."""
        path = self._get_session_path(session.key)

        with open(path, "w") as f:
            # Write metadata first
            metadata_line = {
                "_type": "metadata",
                "created_at": session.created_at.isoformat(),
                "updated_at": session.updated_at.isoformat(),
                "metadata": session.metadata,
            }
            f.write(json.dumps(metadata_line) + "\n")

            # Write messages
            for msg in session.messages:
                f.write(json.dumps(msg) + "\n")

        self._cache[session.key] = session

    def delete(self, key: str) -> bool:
        """
        Delete a session.

        Args:
            key: Session key.

        Returns:
            True if deleted, False if not found.
        """
        # Remove from cache
        self._cache.pop(key, None)

        # Remove file
        path = self._get_session_path(key)
        if path.exists():
            path.unlink()
            return True
        return False

    def list_sessions(self) -> list[dict[str, Any]]:
        """
        List all sessions.

        Returns:
            List of session info dicts.
        """
        sessions = []

        for path in self.sessions_dir.glob("*.jsonl"):
            try:
                # Read just the metadata line
                with open(path) as f:
                    first_line = f.readline().strip()
                    if first_line:
                        data = json.loads(first_line)
                        if data.get("_type") == "metadata":
                            sessions.append(
                                {
                                    "key": path.stem.replace("_", ":"),
                                    "created_at": data.get("created_at"),
                                    "updated_at": data.get("updated_at"),
                                    "path": str(path),
                                }
                            )
            except Exception:
                continue

        return sorted(sessions, key=lambda x: x.get("updated_at", ""), reverse=True)

    def compact_session(self, key: str, keep_last: int = 50, instruction: str | None = None) -> int:
        """Helper: load-or-create the session, run compaction, save, and return compacted count."""
        session = self.get_or_create(key)
        compacted = session.compact(keep_last=keep_last, instruction=instruction)
        if compacted > 0:
            self.save(session)
        return compacted
