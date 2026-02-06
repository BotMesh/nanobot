"""Memory system module - Rust implementation with Python fallback."""

try:
    from nanobot_rust import MemoryStore
except ImportError:
    from nanobot.agent._memory_py import MemoryStore

__all__ = ["MemoryStore"]


def search_memory(
    workspace,
    query: str,
    max_results: int = 5,
    min_score: float = 0.0,
    build_index_if_missing: bool = True,
):
    """Helper: construct `MemoryStore`, ensure index, and run a semantic search.

    Returns a list of dicts with keys `path`, `snippet`, and `score`.
    """
    store = MemoryStore(workspace)

    # If index file missing and builder is available, build it
    # Some implementations expose build_index as a method
    if build_index_if_missing:
        try:
            if hasattr(store, "build_index"):
                # Attempt to load index file path from Python object if available
                mem_dir = store.memory_dir if hasattr(store, "memory_dir") else None
                index_file = None
                if mem_dir:
                    from pathlib import Path

                    index_file = Path(mem_dir) / ".index.json"
                if index_file is None or not index_file.exists():
                    try:
                        store.build_index()
                    except Exception:
                        # ignore build failures and continue to search (may still work)
                        pass
        except Exception:
            pass

    # Call search and normalize results to plain Python list
    try:
        results = store.search(query, max_results, float(min_score))
        # If this is a PyList-like, convert to regular list
        try:
            return list(results)
        except Exception:
            return results
    except Exception:
        return []
