"""
Maps LLM tool call names to CodeIO gRPC service calls.
The AI calls tools by name; this dispatcher routes each to the right service.

Tool registry:
  spotlight      → Rust SpotlightService
  get_context    → Rust ContextWindowService
  code_scan      → Python CodeScanService (wraps type checker + heuristics)
  logic_check    → Haskell TypeCheckService
  dependency_map → Rust DependencyMapService
  read_file      → Direct filesystem read
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable

from ..services.spotlight import SpotlightClient
from ..context.window_manager import WindowManager, TokenBudget


class ToolDispatcher:
    """Converts an AI tool call (name + JSON input) into a service response."""

    def __init__(
        self,
        workspace_id: str,
        pool=None,
    ) -> None:
        self.workspace_id = workspace_id
        self._spotlight = SpotlightClient(pool)
        self._context = WindowManager(pool)
        self._tools: dict[str, Callable[[dict], Any]] = {
            "spotlight":      self._tool_spotlight,
            "get_context":    self._tool_get_context,
            "read_file":      self._tool_read_file,
            "dependency_map": self._tool_dependency_map,
        }

    def dispatch(self, tool_name: str, input_json: str) -> str:
        """Execute a tool call and return the result as a JSON string."""
        params = json.loads(input_json) if input_json else {}
        handler = self._tools.get(tool_name)
        if handler is None:
            return json.dumps({"error": f"Unknown tool: {tool_name}"})
        try:
            result = handler(params)
            return json.dumps(result)
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def available_tools(self) -> list[dict]:
        """Returns the tool definitions to pass to the LLM."""
        return [
            {
                "name": "spotlight",
                "description": (
                    "Fast fuzzy/semantic search over the codebase. "
                    "Use this to find symbols, functions, files, or any string in the workspace."
                ),
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "mode": {"type": "string", "enum": ["EXACT", "FUZZY", "SEMANTIC", "HYBRID"],
                                 "default": "HYBRID"},
                        "scope": {"type": "string", "enum": ["ALL", "SYMBOLS", "FILES", "COMMENTS"],
                                  "default": "ALL"},
                        "max_results": {"type": "integer", "default": 20},
                    },
                    "required": ["query"],
                },
            },
            {
                "name": "get_context",
                "description": (
                    "Retrieve ranked code context around a file position or query. "
                    "Returns the most relevant code within the token budget. "
                    "Use this before answering questions about specific code."
                ),
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query":     {"type": "string"},
                        "file_path": {"type": "string"},
                        "line":      {"type": "integer"},
                        "max_tokens": {"type": "integer", "default": 40000},
                    },
                },
            },
            {
                "name": "read_file",
                "description": "Read the full contents of a specific file.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Workspace-relative or absolute file path"},
                        "start_line": {"type": "integer", "default": 0},
                        "end_line":   {"type": "integer", "default": -1},
                    },
                    "required": ["path"],
                },
            },
            {
                "name": "dependency_map",
                "description": "Return the import/dependency graph for a file.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string"},
                        "max_depth": {"type": "integer", "default": 2},
                    },
                    "required": ["file_path"],
                },
            },
        ]

    # ── Tool implementations ──────────────────────────────────────────────────

    def _tool_spotlight(self, params: dict) -> dict:
        results = self._spotlight.search(
            self.workspace_id,
            params["query"],
            mode=params.get("mode", "HYBRID"),
            scope=params.get("scope", "ALL"),
            max_results=int(params.get("max_results", 20)),
        )
        return {
            "results": [
                {
                    "file": r.file_path,
                    "lines": f"{r.start_line}-{r.end_line}",
                    "score": round(r.score, 3),
                    "kind": r.match_kind,
                    "symbol": r.symbol_name,
                    "context": r.context_before + [">>> match <<<"] + r.context_after,
                }
                for r in results
            ],
            "total": len(results),
        }

    def _tool_get_context(self, params: dict) -> dict:
        budget = TokenBudget(max_tokens=int(params.get("max_tokens", 40_000)))
        ctx = self._context.assemble(
            self.workspace_id,
            file_path=params.get("file_path"),
            line=int(params.get("line", 0)),
            query=params.get("query", ""),
            budget=budget,
        )
        return {
            "context": ctx.render(),
            "total_tokens": ctx.total_tokens,
            "truncated": ctx.truncated,
            "truncation_summary": ctx.truncation_summary,
        }

    def _tool_read_file(self, params: dict) -> dict:
        path = Path(params["path"])
        if not path.exists():
            return {"error": f"File not found: {params['path']}"}
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        start = int(params.get("start_line", 0))
        end   = int(params.get("end_line", -1))
        if end == -1:
            end = len(lines)
        selected = lines[start:end]
        return {
            "path":    str(path),
            "lines":   f"{start}-{end}",
            "content": "\n".join(selected),
            "total_lines": len(lines),
        }

    def _tool_dependency_map(self, params: dict) -> dict:
        # Full implementation requires calling the Rust DepMap gRPC service.
        # For now, extract imports directly from the file.
        path = Path(params["file_path"])
        if not path.exists():
            return {"error": f"File not found: {params['file_path']}"}
        source = path.read_text(encoding="utf-8", errors="replace")
        imports = [
            line.strip()
            for line in source.splitlines()
            if line.strip().startswith(("import ", "from ", "use ", "extern crate", "#include"))
        ]
        return {"file": str(path), "imports": imports[:50]}
