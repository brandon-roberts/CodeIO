"""Python client for the Rust Spotlight gRPC service."""
from __future__ import annotations

from dataclasses import dataclass

# These are generated stubs — available after `./tools/protogen/generate.sh python`
try:
    from proto.gen.python.ai import spotlight_pb2, spotlight_pb2_grpc  # type: ignore
except ImportError:
    spotlight_pb2 = None  # type: ignore
    spotlight_pb2_grpc = None  # type: ignore

from .client_pool import get_pool


@dataclass
class SearchResult:
    file_path: str
    start_line: int
    end_line: int
    score: float
    match_kind: str
    symbol_name: str
    context_before: list[str]
    context_after: list[str]


class SpotlightClient:
    """High-level Python wrapper over the Rust SpotlightService."""

    def __init__(self, pool=None) -> None:
        self._pool = pool or get_pool()

    def search(
        self,
        workspace_id: str,
        query: str,
        *,
        mode: str = "HYBRID",
        scope: str = "ALL",
        languages: list[str] | None = None,
        max_results: int = 20,
        context_lines: int = 3,
    ) -> list[SearchResult]:
        if spotlight_pb2_grpc is None:
            raise RuntimeError(
                "Proto stubs not generated. Run: ./tools/protogen/generate.sh python"
            )

        stub = spotlight_pb2_grpc.SpotlightServiceStub(self._pool.spotlight)
        req = spotlight_pb2.SpotlightQuery(
            workspace_id=workspace_id,
            query_text=query,
            search_mode=getattr(spotlight_pb2.SearchMode, f"SEARCH_MODE_{mode}", 4),
            scope=getattr(spotlight_pb2.SearchScope, f"SEARCH_SCOPE_{scope}", 1),
            max_results=max_results,
            context_lines=context_lines,
        )

        resp = stub.Search(req)
        return [
            SearchResult(
                file_path=h.file_ref.path if h.file_ref else "",
                start_line=h.span.start.line if h.span and h.span.start else 0,
                end_line=h.span.end.line if h.span and h.span.end else 0,
                score=h.score,
                match_kind=spotlight_pb2.MatchKind.Name(h.match_kind).replace("MATCH_KIND_", ""),
                symbol_name=h.symbol_info.name if h.symbol_info else "",
                context_before=list(h.context_before),
                context_after=list(h.context_after),
            )
            for h in resp.hits
        ]
