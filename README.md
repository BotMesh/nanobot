<div align="center">
  <img src="debot_logo.png" alt="debot" width="500">
  <h1>Debot: The Lightweight and Secure OpenClaw</h1>
  <p>
    <a href="https://pypi.org/project/debot/"><img src="https://img.shields.io/pypi/v/debot" alt="PyPI"></a>
    <a href="https://pepy.tech/project/debot"><img src="https://static.pepy.tech/badge/debot" alt="Downloads"></a>
    <a href="https://github.com/BotMesh/debot/actions/workflows/ci.yml"><img src="https://github.com/BotMesh/debot/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/python-≥3.11-blue" alt="Python">
    <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
  </p>
</div>

🐈 **Debot** is a **lightweight** and **secure** personal AI assistant inspired by [Clawdbot](https://github.com/openclaw/openclaw) and [Nanobot](https://github.com/HKUDS/nanobot). 


## Key Features of Debot:
🛡️ **Secure by Design**: Rust for core agent implementation, and minimal dependencies reduce attack surface and vulnerabilities.

💰 **Extremely Token-Saving**: Built-in intelligent router analyzes prompt complexity and automatically selects the cheapest suitable model — **~71% cost reduction** vs. always using a top-tier model.

🪶 **Ultra-Lightweight**: About ~10.8k lines of Rust + Python code (excluding tests) — still far smaller than typical monolithic agents.

🔬 **Research-Ready**: Clean, readable code that's easy to understand, modify, and extend for research.

⚡️ **Lightning Fast**: Minimal footprint means faster startup, lower resource usage, and quicker iterations.


### CI / Docker notes for building the Rust extension

When building the Rust Python extension inside CI or containers on newer Python versions (for example Python 3.14), set the following environment variable so PyO3 uses the stable ABI forward-compatibility:

```bash
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
```

If you need to specify a particular Python executable for maturin builds, set `PYO3_PYTHON` to the interpreter path.
💎 **Easy-to-Use**: One-click to deploy and you're ready to go.

## 🏗️ Architecture

<p align="center">
  <img src="debot_arch.png" alt="debot architecture" width="800">
</p>

## ✨ Features

<table align="center">
  <tr align="center">
    <th><p align="center">📈 24/7 Real-Time Market Analysis</p></th>
    <th><p align="center">🚀 Full-Stack Software Engineer</p></th>
    <th><p align="center">📅 Smart Daily Routine Manager</p></th>
    <th><p align="center">📚 Personal Knowledge Assistant</p></th>
  </tr>
  <tr>
    <td align="center"><p align="center"><img src="case/search.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/code.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/scedule.gif" width="180" height="400"></p></td>
    <td align="center"><p align="center"><img src="case/memory.gif" width="180" height="400"></p></td>
  </tr>
  <tr>
    <td align="center">Discovery • Insights • Trends</td>
    <td align="center">Develop • Deploy • Scale</td>
    <td align="center">Schedule • Automate • Organize</td>
    <td align="center">Learn • Memory • Reasoning</td>
  </tr>
</table>

### Core Capabilities

| Category | What Debot Can Do |
|----------|-------------------|
| ✍️ **Writing & Communication** | AI text humanization, content summarization, natural and human-like output |
| 💻 **Software Engineering** | Test-driven development, systematic debugging, code review, git worktree management |
| 🧠 **Planning & Design** | Brainstorming, implementation planning, subagent-driven parallel execution |
| 🔍 **Research & Analysis** | Web search, real-time market analysis, URL and video summarization |
| 📅 **Task Management** | Daily routines, scheduled tasks (cron), workflow automation |
| 📚 **Knowledge & Memory** | Long-term memory, semantic search, personal knowledge base |

## 📦 Install

**Install from source** (latest features, recommended for development)

> [!NOTE]
> Requires **Python ≥ 3.11** and a **Rust toolchain** (for the native extension). On Linux you also need `patchelf` (`pip install patchelf`).

```bash
git clone https://github.com/BotMesh/debot.git
cd debot
python3 -m venv .venv
source .venv/bin/activate
pip install .
```

**Install with [uv](https://github.com/astral-sh/uv)** (stable, fast)

```bash
uv tool install debot
```

**Install from PyPI** (stable)

```bash
pip install debot
```

## 🚀 Quick Start

> [!TIP]
> Set your API key in `~/.debot/config.json`.
> Get API keys: [OpenRouter](https://openrouter.ai/keys) (LLM) · [Brave Search](https://brave.com/search/api/) (optional, for web search)
> You can also change the model to `minimax/minimax-m2` for lower cost.

**1. Initialize**

```bash
debot onboard
```

**2. Configure** (`~/.debot/config.json`)

```json
{
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-v1-xxx"
    }
  },
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5"
    }
  },
  "webSearch": {
    "apiKey": "BSA-xxx"
  }
}
```


**3. Chat**

```bash
debot agent -m "What is 2+2?"
```

That's it! You have a working AI assistant in 2 minutes.

## 🖥️ Local Models (vLLM)

Run Debot with your own local models using vLLM or any OpenAI-compatible server.

**1. Start your vLLM server**

```bash
vllm serve meta-llama/Llama-3.1-8B-Instruct --port 8000
```

**2. Configure** (`~/.debot/config.json`)

```json
{
  "providers": {
    "vllm": {
      "apiKey": "dummy",
      "apiBase": "http://localhost:8000/v1"
    }
  },
  "agents": {
    "defaults": {
      "model": "meta-llama/Llama-3.1-8B-Instruct"
    }
  }
}
```

**3. Chat**

```bash
debot agent -m "Hello from my local LLM!"
```

## 💾 Session Compaction

Debot automatically compacts long conversations to keep context windows efficient. When a conversation exceeds ~90% of the model's context window, old messages are summarized into a single "compaction" entry.

**Features:**
- ✅ **Automatic** — Triggered silently when context limit approached
- ✅ **Manual** — Use `/compact` command in Telegram or CLI
- ✅ **Configurable** — Tune per-model or globally
- ✅ **Tracked** — View compaction stats in session metadata

**Usage:**

```bash
# Manual compaction via CLI
debot sessions compact telegram:12345 --keep-last 50

# View/configure compaction settings
debot config compaction --show
debot config compaction --keep-last 30 --trigger-ratio 0.85

# Per-model settings
debot config compaction-model "anthropic/claude-opus-4-5" --keep-last 40
```

**Telegram:**

```
/compact              # Use default keep-last=50
/compact 30           # Keep last 30 messages
/compact 30 --verbose # Show detailed results
```

## 🚀 Intelligent Model Router

Debot includes a **built-in intelligent router** (powered by Rust) that automatically selects the best LLM model based on task complexity. This saves costs by routing simple queries to cheaper models while reserving powerful models for complex reasoning tasks.

**How it works:**
- Analyzes incoming prompts across 10 dimensions: reasoning difficulty, code complexity, multi-step reasoning, token count, creativity, technical depth, and more.
- Scores each dimension using heuristic patterns and keyword detection.
- Maps the overall complexity score to a tier: `SIMPLE` → `MEDIUM` → `COMPLEX` → `REASONING`.
- Routes to the configured model for that tier (customizable).

**Default tier-to-model mapping:**
| Tier | Model | Cost (per 1M tokens) |
|------|-------|------|
| `SIMPLE` | `openai/gpt-3.5-turbo` | $1.50 |
| `MEDIUM` | `openai/gpt-4o-mini` | $0.60 |
| `COMPLEX` | `anthropic/claude-opus-4-5` | $25.00 |
| `REASONING` | `openai/o3` | $8.00 |

The router runs automatically — no configuration needed. You can customize the tier-to-model mapping by editing the Rust router config (see `rust/src/router/config.rs`).

**Cost savings benchmark:**

We ran 33 representative prompts (greetings, code tasks, architecture design, formal proofs) through the router and simulated a typical daily workload of 70 queries (see `experiments/router_cost_savings.py`):

| Scenario | Daily Cost | Savings |
|----------|-----------|---------|
| Baseline (always `claude-opus-4-5`) | $0.4285 | — |
| Auto router (current, 58% accuracy) | $0.1249 | **70.8%** |
| Ideal router (100% accuracy) | $0.1990 | 53.6% |

> The router distributes traffic across all 4 tiers: ~58% SIMPLE, ~21% MEDIUM, ~15% COMPLEX, ~6% REASONING. Simple queries ($1.50/M) and medium tasks ($0.60/M) avoid the $25.00/M baseline cost, cutting the bill by ~71%.

**Router CLI tools:**

```bash
# Test how the router scores any prompt
debot router test "implement a distributed cache with consistent hashing"

# View accumulated routing metrics (in long-running sessions)
debot router metrics
```

## 🧠 Long-term memory

Debot stores persistent memory under your workspace at `memory/` (by default your workspace is `~/.debot/workspace`). The memory system supports:

- `MEMORY.md` — long-term notes you want the agent to remember.
- `YYYY-MM-DD.md` — daily notes.
- `.index.json` — a simple local semantic index (auto-generated).

How it works
- The Rust extension (or the Python fallback) exposes `MemoryStore.build_index()` and `MemoryStore.search(query, max_results, min_score)` to build a local vector index and search it.
- If `OPENAI_API_KEY` or `OPENROUTER_API_KEY` is set, Debot will attempt to use the remote embeddings API and fall back to a deterministic local embedding when not available.

Quick enable & usage

1. Build and install the Rust extension (in development environments with Python ≧ 3.14 you may need to set `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`):

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install .          # builds the Rust extension automatically via maturin
```

> [!TIP]
> On Python ≥ 3.14 you may need `export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` before the install.
> On Linux, install `patchelf` first: `pip install patchelf`.

2. Optionally provide an embeddings key (recommended for better results):

```bash
export OPENAI_API_KEY="sk-..."
# or
export OPENROUTER_API_KEY="or-..."
```

3. Build index and search (Python example):

```python
from pathlib import Path
from debot.agent.memory import search_memory, MemoryStore
# Build index explicitly (if you've updated memory files)
store = MemoryStore(ws)
store.build_index()

# Search
results = search_memory(ws, "when did I last deploy?", max_results=5)
for r in results:
  print(r["score"], r["path"])
  print(r["snippet"][:200])
  print("---")
```

Notes
- If the `.index.json` file is missing, `search_memory()` will attempt to call `build_index()` automatically.
- The local deterministic embedding is SHA256-based and works offline but yields lower-quality semantic matches than remote embeddings.


> [!TIP]
> The `apiKey` can be any non-empty string for local servers that don't require authentication.

## 💬 Chat Apps

Talk to your Debot through Telegram or WhatsApp — anytime, anywhere.

| Channel | Setup |
|---------|-------|
| **Telegram** | Easy (just a token) |
| **WhatsApp** | Medium (scan QR) |

<details>
<summary><b>Telegram</b> (Recommended)</summary>

**1. Create a bot**
- Open Telegram, search `@BotFather`
- Send `/newbot`, follow prompts
- Copy the token

**2. Configure**

```json
{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "YOUR_BOT_TOKEN",
      "allowFrom": ["YOUR_USER_ID"]
    }
  }
}
```

> Get your user ID from `@userinfobot` on Telegram.

**3. Run**

```bash
debot gateway
```

</details>

<details>
<summary><b>WhatsApp</b></summary>

Requires **Node.js ≥18**.

**1. Link device**

```bash
debot channels login
# Scan QR with WhatsApp → Settings → Linked Devices
```

**2. Configure**

```json
{
  "channels": {
    "whatsapp": {
      "enabled": true,
      "allowFrom": ["+1234567890"]
    }
  }
}
```

**3. Run** (two terminals)

```bash
# Terminal 1
debot channels login

# Terminal 2
debot gateway
```

</details>

## 🎯 Built-in Skills

debot comes with **21 built-in skills** covering the full development and writing lifecycle:

**Development Workflow**

| Skill | Description |
|-------|-------------|
| **brainstorming** 🧠 | Turn ideas into fully formed designs and specs through collaborative dialogue |
| **writing-plans** 📝 | Create comprehensive implementation plans with bite-sized tasks |
| **executing-plans** ▶️ | Execute plans with review checkpoints between batches |
| **subagent-driven-development** 🤖 | Dispatch independent subagents per task with two-stage review |
| **dispatching-parallel-agents** 🔀 | Run 2+ independent tasks in parallel across agents |
| **finishing-a-development-branch** 🏁 | Guide branch completion — merge, PR, or cleanup |

**Code Quality & Review**

| Skill | Description |
|-------|-------------|
| **test-driven-development** 🧪 | Write tests first, watch them fail, implement minimal code to pass |
| **systematic-debugging** 🔍 | Four-phase root cause investigation before attempting fixes |
| **verification-before-completion** ✅ | Run verification commands and confirm output before claiming done |
| **requesting-code-review** 📤 | Dispatch code-reviewer subagent to catch issues early |
| **receiving-code-review** 📥 | Evaluate review feedback with rigor before implementing |

**Writing & Communication**

| Skill | Description |
|-------|-------------|
| **humanizer** ✍️ | Remove AI writing patterns to produce natural, human-like text |
| **summarize** 📄 | Summarize URLs, files, and YouTube videos |

**Tools & Infrastructure**

| Skill | Description |
|-------|-------------|
| **github** 🐙 | Interact with GitHub using the `gh` CLI — PRs, issues, CI runs, and queries |
| **weather** ⛅ | Get weather info using wttr.in and Open-Meteo APIs |
| **tmux** 🖥️ | Remote-control tmux sessions for terminal automation |
| **using-git-worktrees** 🌳 | Create isolated git worktrees for feature work |

**Skill Management**

| Skill | Description |
|-------|-------------|
| **skill-creator** 🔧 | Create and package new custom skills |
| **writing-skills** 📖 | TDD-driven skill development and editing |
| **find-skills** 🔎 | Discover available skills in workspace and system |

**Usage:**

```bash
# List available skills
debot skills list

# List system (built-in) and workspace skills as JSON
debot skills list --json

# Install a system skill to your workspace
debot skills install github
debot skills install weather

# Filter skills by name
debot skills list --query github
```

**Create a custom skill:**

Each skill is a directory with a `SKILL.md` file containing YAML frontmatter and instructions:

```yaml
---
name: my-skill
description: "A custom skill that does X"
metadata: {"debot": {"emoji": "✨", "requires": {"bins": ["tool"]}}}
---

# My Custom Skill

Instructions for the agent on how to use this skill...
```

Place your skill in `~/.debot/workspace/skills/<skill-name>/SKILL.md` and it will be automatically available to your agent.

## ⚙️ Configuration

Config file: `~/.debot/config.json`

### Providers

> [!NOTE]
> Groq provides free voice transcription via Whisper. If configured, Telegram voice messages will be automatically transcribed.

| Provider | Purpose | Get API Key |
|----------|---------|-------------|
| `openrouter` | LLM (recommended, access to all models) | [openrouter.ai](https://openrouter.ai) |
| `anthropic` | LLM (Claude direct) | [console.anthropic.com](https://console.anthropic.com) |
| `openai` | LLM (GPT direct) | [platform.openai.com](https://platform.openai.com) |
| `groq` | LLM + **Voice transcription** (Whisper) | [console.groq.com](https://console.groq.com) |
| `gemini` | LLM (Gemini direct) | [aistudio.google.com](https://aistudio.google.com) |


<details>
<summary><b>Full config example</b></summary>

```json
{
  "agents": {
    "defaults": {
      "model": "anthropic/claude-opus-4-5"
    }
  },
  "providers": {
    "openrouter": {
      "apiKey": "sk-or-v1-xxx"
    },
    "groq": {
      "apiKey": "gsk_xxx"
    }
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "123456:ABC...",
      "allowFrom": ["123456789"]
    },
    "whatsapp": {
      "enabled": false
    }
  },
  "tools": {
    "web": {
      "search": {
        "apiKey": "BSA..."
      }
    }
  }
}
```

</details>

## CLI Reference

| Command | Description |
|---------|-------------|
| `debot onboard` | Initialize config & workspace |
| `debot agent -m "..."` | Chat with the agent |
| `debot agent` | Interactive chat mode |
| `debot gateway` | Start the gateway |
| `debot status` | Show status |
| `debot channels login` | Link WhatsApp (scan QR) |
| `debot channels status` | Show channel status |
| `debot sessions compact <key>` | Manually compact a session |
| `debot config compaction` | View/configure compaction settings |
| `debot config compaction-model <model>` | Set per-model compaction settings |

<details>
<summary><b>Scheduled Tasks (Cron)</b></summary>

```bash
# Add a job
debot cron add --name "daily" --message "Good morning!" --cron "0 9 * * *"
debot cron add --name "hourly" --message "Check status" --every 3600

# List jobs
debot cron list

# Remove a job
debot cron remove <job_id>
```

</details>

## 🐳 Docker

> [!TIP]
> The `-v ~/.debot:/root/.debot` flag mounts your local config directory into the container, so your config and workspace persist across container restarts.

### Build & Run Locally

Build and run debot in a container:

```bash
# Build the image
docker build -t debot .

# Initialize config (first time only)
docker run -v ~/.debot:/root/.debot --rm debot onboard

# Edit config on host to add API keys
vim ~/.debot/config.json

# Run gateway (connects to Telegram/WhatsApp)
docker run -v ~/.debot:/root/.debot -p 18790:18790 debot gateway

# Or run a single command
docker run -v ~/.debot:/root/.debot --rm debot agent -m "Hello!"
docker run -v ~/.debot:/root/.debot --rm debot status
```

### 📦 Pull from GitHub Container Registry

Pre-built images are automatically published to GitHub Container Registry:

```bash
# Pull latest image
docker pull ghcr.io/BotMesh/debot:latest

# Run with pulled image
docker run -v ~/.debot:/root/.debot -p 18790:18790 ghcr.io/BotMesh/debot:latest gateway

# Pull specific version
docker pull ghcr.io/BotMesh/debot:v1.0.0
```

**Available Tags:**
- `latest` — Latest main branch
- `main` — Main branch  
- `v1.0.0` — Release versions
- `main-<short-sha>` — Specific commits

For more info, see [Container Publishing Guide](./.github/CONTAINER_PUBLISHING.md)


## 🤝 Contribute & Roadmap

PRs welcome! The codebase is intentionally small and readable. 🤗

**Roadmap** — Pick an item and [open a PR](https://github.com/BotMesh/debot/pulls)!

- [x] **Voice Transcription** — Support for Groq Whisper (Issue #13)
- [ ] **Multi-modal** — See and hear (images, voice, video)
- [x] **Intelligent Model Router** — Automatically selects the best LLM model based on task complexity
- [x] **Long-term memory** — Never forget important context
- [ ] **Better reasoning** — Multi-step planning and reflection
- [ ] **More integrations** — Discord, Slack, email, calendar
- [ ] **Self-improvement** — Learn from feedback and mistakes

<p align="center">
  <sub>debot is for educational, research, and technical exchange purposes only</sub>
</p>
