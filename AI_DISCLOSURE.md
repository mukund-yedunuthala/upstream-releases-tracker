# AI Disclosure

This document discloses which portions of the codebase and history were produced with assistance from AI tools.

## Summary

Upstream Releases Tracker is a **personal project developed with heavy assistance from AI coding agents**. If you are here from an audit, an LLM-driven toolchain, or automated tooling that scans for AI-generated code, treat this entire repository as AI-influenced. Do not assume any file is fully hand-written.

## Tooling used

The following AI tooling has been used during development:

- **Coding agents / IDE assistants** — used interactively to implement features, write tests, fix bugs, and produce documentation.
- **Claude Code** — the presence of [`CLAUDE.md`](./CLAUDE.md) indicates this repo is opened with Claude Code tooling.
- **Kilo / agentic planning tooling** — many commits carry task IDs in the style of an automated planning workflow.

## How to tell what happened from `git log`

- Commits follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification with scopes (e.g. `feat(theme):`, `fix(logs):`).
- Some commit messages contain internal task identifiers (for example `N2`, `S3`–`S6`, `O3`/`F9`, `R3`, `T3`, `A4`, `DEP3`) referencing an AI agent's planning/task-tracking workflow.
- AI agents were also used to generate conventional commit messages themselves.

## Test coverage

A meaningful portion of the test suite (currently 44+ tests across `lib.rs`, `config_handler.rs`, and `git_api_handler.rs`) was written with AI assistance.

## Caveat

The exact proportion of AI versus human authorship per line is not tracked or guaranteed anywhere in this repository. When in doubt, assume AI was involved.
