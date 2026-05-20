# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this app does

Upstream Releases Tracker is a Tauri 2.x desktop app that lets users track the latest release tags from GitHub, Forgejo/Gitea, and (future) GitLab repositories, and compare them against the version installed on their system.

## Development commands

Install frontend dependencies first (only needed once or after `package.json` changes):
```sh
cd frontend && npm install
```

Run in dev mode (starts both the Vite dev server and the Tauri shell):
```sh
cd src-tauri && cargo tauri dev
```

Build a distributable bundle:
```sh
cd src-tauri && cargo tauri build
```

Run Rust tests:
```sh
cargo test --workspace
```

Frontend-only dev (no Tauri, just the Vite server at `localhost:1420`):
```sh
cd frontend && npm run dev
```

## Architecture

### Crate layout

The workspace has three crates:

- **`tracker_libs/`** — shared types only: `Config`, `RepoData`, `ForgeKind`. No I/O or async. Both the Tauri backend and the root crate depend on this.
- **`src-tauri/`** — the Tauri backend. Exposes six Tauri commands to the frontend and owns all I/O and HTTP logic.
- **Root crate** (`src/main.rs`) — legacy entry point from the original Yew WASM frontend, superseded by the `frontend/` directory. Not actively developed.

### Backend modules (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Tauri command definitions and the `run()` entry point |
| `git_api_handler.rs` | HTTP calls to forge APIs; parses repo URLs; dispatches by `ForgeKind` |
| `app_content_handler.rs` | CRUD on `repos.json` (read, add, delete, update status) |
| `config_handler.rs` | Read/write `config.json`; creates a default template on first run |
| `json_handler.rs` | Thin wrapper for serde_json file I/O |
| `migration.rs` | Idempotent schema migrations for `repos.json` and `config.json`; runs at every startup |
| `helper.rs` | Thin bridge so `lib.rs` can call `ConfigHandler` without importing it directly |

### Frontend (`frontend/`)

Vanilla JS + Vite, no framework. `app.js` is the single JS file. The UI library is `oat` (bundled as `oat.min.css` / `oat.min.js`). The frontend communicates with the Tauri backend exclusively through `invoke()` calls from `@tauri-apps/api/core`.

### Data files (Linux paths)

- Repos: `~/.local/share/upstream-releases-tracker/data/repos.json`
- Config: `~/.config/upstream-releases-tracker/config.json` — API tokens live here; created automatically with empty values on first run.

### Tauri commands

`get_repos`, `add_repo`, `delete_repo`, `refresh_repo`, `mark_as_updated` — all defined in `src-tauri/src/lib.rs`.

### Adding a new forge

1. Add a variant to `ForgeKind` in `tracker_libs/src/lib.rs`.
2. Add a branch in the `api_call` match in `src-tauri/src/git_api_handler.rs`.
3. Add a config field to `Config` in `tracker_libs/src/lib.rs` and update `Config::new()`.
4. Add a migration in `migration.rs` to backfill the new config field.
5. Add the option to the `<select>` in `frontend/index.html` and the `forgeLabel` switch in `frontend/app.js`.

GitLab is implemented as of 3.4.0. Subgroup URL support (≥3 path segments) was added in the Batch 10c audit fixes.

### URL validation

`isValidRepoUrl` in `frontend/app.js` (regex pre-check for instant UX feedback) and `parse_url` in `src-tauri/src/git_api_handler.rs` (authoritative backend validation via `url::Url::parse`) must stay in sync — both enforce HTTPS URLs with at least two path segments (`host/owner/repo`). GitLab subgroup URLs with additional segments (`host/group/subgroup/project`) are valid; `parse_url` joins all segments except the last as the owner.
