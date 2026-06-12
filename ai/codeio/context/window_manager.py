"""
Assembles a ContextWindow for an AI call by calling the Rust context service.
Handles the mmap protocol for large windows.
"""
from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterator

try:
    from proto.gen.python.ai import (  # type: ignore
        context_window_pb2,
        context_window_pb2_grpc,
        types_pb2,
    )
    from proto.gen.python.core import common_pb2  # type: ignore
except ImportError:
    context_window_pb2 = None  # type: ignore
    context_window_pb2_grpc = None  # type: ignore
    types_pb2 = None  # type: ignore
    common_pb2 = None  # type: ignore

from ..services.client_pool import get_pool


@dataclass
class TokenBudget:
    max_tokens: int = 100_000
    system_reserve: int = 4_000
    history_reserve: int = 8_000
    response_reserve: int = 4_000

    @property
    def context_tokens(self) -> int:
        return self.max_tokens - self.system_reserve - self.history_reserve - self.response_reserve


@dataclass
class ContextSlice:
    file_path: str
    start_line: int
    end_line: int
    raw_content: str
    relevance_score: float
    include_reason: str
    symbol_name: str = ""
    tokens: int = 0


@dataclass
class AssembledContext:
    workspace_id: str
    slices: list[ContextSlice] = field(default_factory=list)
    total_tokens: int = 0
    truncated: bool = False
    truncation_summary: str = ""

    def render(self) -> str:
        """Render context as a string suitable for insertion into an AI prompt."""
        parts = []
        for s in self.slices:
            header = f"# {s.file_path}:{s.start_line}-{s.end_line}"
            if s.symbol_name:
                header += f"  ({s.symbol_name})"
            parts.append(f"{header}\n```\n{s.raw_content}\n```")
        return "\n\n".join(parts)

    def iter_by_file(self) -> Iterator[tuple[str, list[ContextSlice]]]:
        files: dict[str, list[ContextSlice]] = {}
        for s in self.slices:
            files.setdefault(s.file_path, []).append(s)
        yield from files.items()


class WindowManager:
    """Assembles context windows by calling the Rust ContextWindowService."""

    def __init__(self, pool=None) -> None:
        self._pool = pool or get_pool()

    def assemble(
        self,
        workspace_id: str,
        *,
        file_path: str | None = None,
        line: int = 0,
        query: str = "",
        budget: TokenBudget | None = None,
    ) -> AssembledContext:
        if context_window_pb2_grpc is None:
            raise RuntimeError("Proto stubs not generated. Run: ./tools/protogen/generate.sh python")

        budget = budget or TokenBudget()
        stub = context_window_pb2_grpc.ContextWindowServiceStub(self._pool.context)

        focus = types_pb2.FocusPoint(query=query)
        if file_path:
            focus.file_ref.CopyFrom(common_pb2.FileRef(path=file_path, workspace_id=workspace_id))
            focus.position.CopyFrom(common_pb2.Position(line=line))

        proto_budget = types_pb2.ContextBudget(
            max_tokens=budget.max_tokens,
            system_reserve=budget.system_reserve,
            history_reserve=budget.history_reserve,
            response_reserve=budget.response_reserve,
        )

        assemble_resp = stub.AssembleWindow(context_window_pb2.AssembleWindowRequest(
            workspace_id=workspace_id,
            focus=focus,
            budget=proto_budget,
        ))

        # Verify checksum before deserializing
        data = Path(assemble_resp.mmap_path).read_bytes()
        actual_checksum = hashlib.sha256(data).hexdigest()
        if actual_checksum != assemble_resp.checksum:
            raise RuntimeError(f"Context window checksum mismatch: expected {assemble_resp.checksum}")

        window = context_window_pb2.ContextWindow()
        window.ParseFromString(data)

        slices = [
            ContextSlice(
                file_path=s.entry.file_ref.path if s.entry and s.entry.file_ref else "",
                start_line=s.entry.span.start.line if s.entry and s.entry.span and s.entry.span.start else 0,
                end_line=s.entry.span.end.line if s.entry and s.entry.span and s.entry.span.end else 0,
                raw_content=s.entry.raw_content if s.entry else "",
                relevance_score=s.relevance_score,
                include_reason=str(s.include_reason),
                symbol_name=s.entry.symbol_record.name if s.entry and s.entry.symbol_record else "",
                tokens=s.entry.tokens if s.entry else 0,
            )
            for s in window.slices
        ]

        return AssembledContext(
            workspace_id=workspace_id,
            slices=slices,
            total_tokens=window.total_tokens,
            truncated=window.truncated,
            truncation_summary=window.truncation_summary,
        )
