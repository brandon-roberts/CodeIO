"""
Unified LLM client for CodeIO AI.
Wraps the Anthropic SDK with CodeIO tool dispatch wired in.
The AI gets the full tool library automatically on every call.
"""
from __future__ import annotations

import os
from collections.abc import Iterator
from typing import Any

import anthropic

from .tool_dispatcher import ToolDispatcher


# Default model — update here when upgrading
DEFAULT_MODEL = "claude-sonnet-4-6"
MAX_TOOL_ROUNDS = 10


class CodeIOClient:
    """
    High-level AI client that combines Anthropic's API with CodeIO's
    tool library. Automatically dispatches tool calls to the right service.

    Usage:
        client = CodeIOClient(workspace_id="/path/to/workspace")
        for chunk in client.query("Find all functions that parse JSON"):
            print(chunk, end="", flush=True)
    """

    def __init__(
        self,
        workspace_id: str,
        *,
        model: str = DEFAULT_MODEL,
        api_key: str | None = None,
        pool=None,
    ) -> None:
        self.workspace_id = workspace_id
        self.model = model
        self._anthropic = anthropic.Anthropic(api_key=api_key or os.environ["ANTHROPIC_API_KEY"])
        self._dispatcher = ToolDispatcher(workspace_id, pool)

    def query(
        self,
        user_message: str,
        *,
        history: list[dict] | None = None,
        system: str | None = None,
        max_tokens: int = 8192,
    ) -> Iterator[str]:
        """
        Stream a response. Automatically handles multi-round tool use.
        Yields text chunks as they arrive.
        """
        messages: list[dict[str, Any]] = list(history or [])
        messages.append({"role": "user", "content": user_message})

        tools = self._dispatcher.available_tools()
        sys_prompt = system or self._default_system()

        for _ in range(MAX_TOOL_ROUNDS):
            with self._anthropic.messages.stream(
                model=self.model,
                max_tokens=max_tokens,
                system=sys_prompt,
                tools=tools,
                messages=messages,
            ) as stream:
                full_response = stream.get_final_message()

            # Collect any text from this response
            for block in full_response.content:
                if block.type == "text":
                    yield block.text

            if full_response.stop_reason != "tool_use":
                break

            # Execute all tool calls and add results
            tool_results = []
            for block in full_response.content:
                if block.type == "tool_use":
                    result = self._dispatcher.dispatch(block.name, block.input if isinstance(block.input, str) else __import__("json").dumps(block.input))
                    tool_results.append({
                        "type": "tool_result",
                        "tool_use_id": block.id,
                        "content": result,
                    })

            messages.append({"role": "assistant", "content": full_response.content})
            messages.append({"role": "user", "content": tool_results})

    def query_sync(self, user_message: str, **kwargs: Any) -> str:
        """Non-streaming version of query. Returns the complete response."""
        return "".join(self.query(user_message, **kwargs))

    def _default_system(self) -> str:
        return (
            "You are an AI software engineer embedded in the CodeIO development environment. "
            "You have access to tools that let you search and read the codebase directly. "
            "Always use the spotlight or get_context tools before answering questions about "
            "specific code — do not make assumptions about code you haven't read. "
            f"Current workspace: {self.workspace_id}"
        )
