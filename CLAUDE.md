# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository

This is a fresh repository (`brandon-roberts/CodeIO`) with no established stack yet. Update this file as the project takes shape.

## Branch Strategy

Active development branch: `claude/environment-optimization-pbll5x`

Push all changes to that branch:
```
git push -u origin claude/environment-optimization-pbll5x
```

## GitHub Integration

Use the `mcp__github__*` MCP tools for all GitHub interactions (PRs, issues, CI, file browsing). The `gh` CLI is not available in this environment.

Scoped to repository: `brandon-roberts/CodeIO`

## Environment Notes

- Running in a remote ephemeral container — commit and push anything worth keeping before the session ends.
- Outbound network is available subject to the environment's policy.
- No `gh` CLI; use GitHub MCP tools instead.
