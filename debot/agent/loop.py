"""Agent loop: the core processing engine."""

import asyncio
import json
from pathlib import Path

from loguru import logger

from debot.agent.context import ContextBuilder
from debot.agent.subagent import SubagentManager
from debot.agent.tools import (
    EditFileTool,
    ExecTool,
    ListDirTool,
    MessageTool,
    ReadFileTool,
    SpawnTool,
    ToolRegistry,
    WebFetchTool,
    WebSearchTool,
    WriteFileTool,
)
from debot.bus import InboundMessage, MessageBus, OutboundMessage
from debot.providers.base import LLMProvider
from debot.session._manager_py import SessionManager


class AgentLoop:
    """
    The agent loop is the core processing engine.

    It:
    1. Receives messages from the bus
    2. Builds context with history, memory, skills
    3. Calls the LLM
    4. Executes tool calls
    5. Sends responses back
    """

    def __init__(
        self,
        bus: MessageBus,
        provider: LLMProvider,
        workspace: Path,
        model: str | None = None,
        max_iterations: int = 20,
        brave_api_key: str | None = None,
    ):
        self.bus = bus
        self.provider = provider
        self.workspace = workspace
        self.model = model or provider.get_default_model()
        self.max_iterations = max_iterations
        self.brave_api_key = brave_api_key

        self.context = ContextBuilder(workspace)
        self.sessions = SessionManager(workspace)
        self.tools = ToolRegistry()
        self.subagents = SubagentManager(
            provider=provider,
            workspace=workspace,
            bus=bus,
            model=self.model,
            brave_api_key=brave_api_key,
        )

        self._running = False
        self._register_default_tools()

    def _register_default_tools(self) -> None:
        """Register the default set of tools."""
        # File tools
        self.tools.register(ReadFileTool())
        self.tools.register(WriteFileTool())
        self.tools.register(EditFileTool())
        self.tools.register(ListDirTool())

        # Shell tool
        self.tools.register(ExecTool(working_dir=str(self.workspace)))

        # Web tools
        self.tools.register(WebSearchTool(api_key=self.brave_api_key))
        self.tools.register(WebFetchTool())

        # Message tool
        message_tool = MessageTool(send_callback=self.bus.publish_outbound)
        self.tools.register(message_tool)

        # Spawn tool (for subagents)
        spawn_tool = SpawnTool(manager=self.subagents)
        self.tools.register(spawn_tool)

    async def run(self) -> None:
        """Run the agent loop, processing messages from the bus."""
        self._running = True
        logger.info("Agent loop started")

        while self._running:
            try:
                # Wait for next message
                msg = await asyncio.wait_for(self.bus.consume_inbound(), timeout=1.0)

                # Process it
                try:
                    response = await self._process_message(msg)
                    if response:
                        await self.bus.publish_outbound(response)
                except Exception as e:
                    logger.error(f"Error processing message: {e}")
                    # Send error response
                    await self.bus.publish_outbound(
                        OutboundMessage(
                            channel=msg.channel,
                            chat_id=msg.chat_id,
                            content=f"Sorry, I encountered an error: {str(e)}",
                        )
                    )
            except asyncio.TimeoutError:
                continue

    def stop(self) -> None:
        """Stop the agent loop."""
        self._running = False
        logger.info("Agent loop stopping")

    async def _process_message(self, msg: InboundMessage) -> OutboundMessage | None:
        """
        Process a single inbound message.

        Args:
            msg: The inbound message to process.

        Returns:
            The response message, or None if no response needed.
        """
        # Handle system messages (subagent announces)
        # The chat_id contains the original "channel:chat_id" to route back to
        if msg.channel == "system":
            return await self._process_system_message(msg)

        logger.info(f"Processing message from {msg.channel}:{msg.sender_id}")

        # Get or create session
        session = self.sessions.get_or_create(msg.session_key)

        # Update tool contexts
        message_tool = self.tools.get("message")
        if isinstance(message_tool, MessageTool):
            message_tool.set_context(msg.channel, msg.chat_id)

        spawn_tool = self.tools.get("spawn")
        if isinstance(spawn_tool, SpawnTool):
            spawn_tool.set_context(msg.channel, msg.chat_id)

        # Build initial messages (use get_history for LLM-formatted messages)
        messages = self.context.build_messages(
            history=session.get_history(),
            current_message=msg.content,
            media=msg.media if msg.media else None,
        )

        # Auto-compaction: estimate prompt size and compact older history if needed.
        try:
            from debot.config.loader import load_config

            config = load_config()
            defaults = config.agents.defaults
            max_tokens = int(getattr(defaults, "max_tokens", 8192) or 8192)
            compaction_enabled = bool(getattr(defaults, "compaction_enabled", True))
            compaction_keep_last = int(getattr(defaults, "compaction_keep_last", 50))
            compaction_trigger_ratio = float(getattr(defaults, "compaction_trigger_ratio", 0.9))
            compaction_silent = bool(getattr(defaults, "compaction_silent", True))
            chars_per_token = int(getattr(defaults, "token_chars_per_token", 4) or 4)

            # Apply model-specific overrides if available
            model_overrides = getattr(defaults, "compaction_model_overrides", {}) or {}
            if self.model in model_overrides:
                override = model_overrides[self.model]
                if override.keep_last is not None:
                    compaction_keep_last = override.keep_last
                if override.trigger_ratio is not None:
                    compaction_trigger_ratio = override.trigger_ratio
                if override.silent is not None:
                    compaction_silent = override.silent
        except Exception:
            # Fallback defaults
            max_tokens = 8192
            compaction_enabled = True
            compaction_keep_last = 50
            compaction_trigger_ratio = 0.9
            compaction_silent = True
            chars_per_token = 4

        if compaction_enabled:
            # Naive token estimate: 1 token ~= chars_per_token characters
            estimated_tokens = sum(len(str(m.get("content", ""))) for m in messages) // max(
                1, chars_per_token
            )
            if estimated_tokens >= int(max_tokens * compaction_trigger_ratio):
                if not compaction_silent:
                    logger.info(
                        f"Context near limit ({estimated_tokens}/{max_tokens} tokens). Running compaction."
                    )
                # Compact the session using configured keep_last
                try:
                    compacted = self.sessions.compact_session(
                        msg.session_key, keep_last=compaction_keep_last
                    )
                    if compacted > 0:
                        # Rebuild messages from the compacted history
                        session = self.sessions.get_or_create(msg.session_key)
                        messages = self.context.build_messages(
                            history=session.get_history(),
                            current_message=msg.content,
                            media=msg.media if msg.media else None,
                        )
                        if not compaction_silent:
                            logger.info(
                                f"Auto-compaction completed: {compacted} messages compacted."
                            )
                except Exception as e:
                    logger.warning(f"Auto-compaction failed: {e}")

        # Agent loop
        iteration = 0
        final_content = None

        while iteration < self.max_iterations:
            iteration += 1

            # Call Rust router (if available) to choose model, then call LLM
            chosen_model = self.model
            current_tier = None
            _debot_rust = None
            try:
                import debot_rust as _debot_rust_mod

                _debot_rust = _debot_rust_mod
                decision_json = _debot_rust.route_text(msg.content, max_tokens)
                if decision_json:
                    try:
                        dec = json.loads(decision_json)
                        chosen_model = dec.get("model", self.model)
                        current_tier = dec.get("tier")
                        logger.info(
                            "Router: tier={} model={} confidence={:.2f} cost=${:.2f}/M",
                            dec.get("tier", "?"),
                            chosen_model,
                            dec.get("confidence", 0),
                            dec.get("cost_estimate", 0),
                        )
                    except Exception:
                        chosen_model = self.model
            except Exception:
                # Router not available or failed; fall back to default model
                chosen_model = self.model

            # Pre-check: escalate if estimated tokens exceed model context window
            if _debot_rust and current_tier:
                try:
                    estimated_tokens = sum(len(m.get("content", "") or "") for m in messages) // 4
                    ctx_limit = _debot_rust.get_context_length(chosen_model)
                    while ctx_limit > 0 and estimated_tokens > int(ctx_limit * 0.9):
                        fb_json = _debot_rust.get_fallback_model(current_tier)
                        if not fb_json:
                            break
                        fb = json.loads(fb_json)
                        logger.warning(
                            "Pre-check: ~{} tokens exceeds {} context ({}), escalating → {} ({})",
                            estimated_tokens,
                            chosen_model,
                            ctx_limit,
                            fb["model"],
                            fb["tier"],
                        )
                        chosen_model = fb["model"]
                        current_tier = fb["tier"]
                        ctx_limit = _debot_rust.get_context_length(chosen_model)
                except Exception:
                    pass  # Pre-check is best-effort

            # Call LLM with escalation on failure
            response = await self.provider.chat(
                messages=messages, tools=self.tools.get_definitions(), model=chosen_model
            )

            # Auto-escalate: if model failed, try next tier (up to 3 escalations)
            if (
                _debot_rust
                and current_tier
                and response.finish_reason in ("error", "context_length_exceeded")
            ):
                for _esc in range(3):
                    fb_json = _debot_rust.get_fallback_model(current_tier)
                    if not fb_json:
                        break
                    fb = json.loads(fb_json)
                    logger.warning(
                        "Escalating: {} ({}) failed [{}] → {} ({})",
                        chosen_model,
                        current_tier,
                        response.finish_reason,
                        fb["model"],
                        fb["tier"],
                    )
                    try:
                        _debot_rust.record_escalation()
                    except Exception:
                        pass
                    chosen_model = fb["model"]
                    current_tier = fb["tier"]
                    response = await self.provider.chat(
                        messages=messages, tools=self.tools.get_definitions(), model=chosen_model
                    )
                    if response.finish_reason not in ("error", "context_length_exceeded"):
                        break

            # Handle tool calls
            if response.has_tool_calls:
                # Add assistant message with tool calls
                tool_call_dicts = [
                    {
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": json.dumps(tc.arguments),  # Must be JSON string
                        },
                    }
                    for tc in response.tool_calls
                ]
                messages = self.context.add_assistant_message(
                    messages, response.content, tool_call_dicts
                )

                # Execute tools
                for tool_call in response.tool_calls:
                    args_str = json.dumps(tool_call.arguments)
                    logger.debug(f"Executing tool: {tool_call.name} with arguments: {args_str}")
                    result = await self.tools.execute(tool_call.name, tool_call.arguments)
                    messages = self.context.add_tool_result(
                        messages, tool_call.id, tool_call.name, result
                    )
            else:
                # No tool calls, we're done
                final_content = response.content
                break

        if final_content is None:
            final_content = "I've completed processing but have no response to give."

        # Save to session
        session.add_message("user", msg.content)
        session.add_message("assistant", final_content)
        self.sessions.save(session)

        return OutboundMessage(channel=msg.channel, chat_id=msg.chat_id, content=final_content)

    async def _process_system_message(self, msg: InboundMessage) -> OutboundMessage | None:
        """
        Process a system message (e.g., subagent announce).

        The chat_id field contains "original_channel:original_chat_id" to route
        the response back to the correct destination.
        """
        logger.info(f"Processing system message from {msg.sender_id}")

        # Parse origin from chat_id (format: "channel:chat_id")
        if ":" in msg.chat_id:
            parts = msg.chat_id.split(":", 1)
            origin_channel = parts[0]
            origin_chat_id = parts[1]
        else:
            # Fallback
            origin_channel = "cli"
            origin_chat_id = msg.chat_id

        # Use the origin session for context
        session_key = f"{origin_channel}:{origin_chat_id}"
        session = self.sessions.get_or_create(session_key)

        # Update tool contexts
        message_tool = self.tools.get("message")
        if isinstance(message_tool, MessageTool):
            message_tool.set_context(origin_channel, origin_chat_id)

        spawn_tool = self.tools.get("spawn")
        if isinstance(spawn_tool, SpawnTool):
            spawn_tool.set_context(origin_channel, origin_chat_id)

        # Build messages with the announce content
        messages = self.context.build_messages(
            history=session.get_history(), current_message=msg.content
        )

        # Agent loop (limited for announce handling)
        iteration = 0
        final_content = None

        while iteration < self.max_iterations:
            iteration += 1

            response = await self.provider.chat(
                messages=messages, tools=self.tools.get_definitions(), model=self.model
            )

            if response.has_tool_calls:
                tool_call_dicts = [
                    {
                        "id": tc.id,
                        "type": "function",
                        "function": {"name": tc.name, "arguments": json.dumps(tc.arguments)},
                    }
                    for tc in response.tool_calls
                ]
                messages = self.context.add_assistant_message(
                    messages, response.content, tool_call_dicts
                )

                for tool_call in response.tool_calls:
                    args_str = json.dumps(tool_call.arguments)
                    logger.debug(f"Executing tool: {tool_call.name} with arguments: {args_str}")
                    result = await self.tools.execute(tool_call.name, tool_call.arguments)
                    messages = self.context.add_tool_result(
                        messages, tool_call.id, tool_call.name, result
                    )
            else:
                final_content = response.content
                break

        if final_content is None:
            final_content = "Background task completed."

        # Save to session (mark as system message in history)
        session.add_message("user", f"[System: {msg.sender_id}] {msg.content}")
        session.add_message("assistant", final_content)
        self.sessions.save(session)

        return OutboundMessage(
            channel=origin_channel, chat_id=origin_chat_id, content=final_content
        )

    async def process_direct(self, content: str, session_key: str = "cli:direct") -> str:
        """
        Process a message directly (for CLI usage).

        Args:
            content: The message content.
            session_key: Session identifier.

        Returns:
            The agent's response.
        """
        msg = InboundMessage(channel="cli", sender_id="user", chat_id="direct", content=content)

        response = await self._process_message(msg)
        return response.content if response else ""
