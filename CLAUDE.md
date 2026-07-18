# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this app does

Upstream Releases Tracker is a Tauri 2.x desktop app that lets users track the latest release tags from GitHub, Forgejo/Gitea, and GitLab repositories, and compare them against the version installed on their system.

## Development commands

Install frontend dependencies first (only needed once or after `package.json` changes):
```sh
npm install
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
npm run dev
```

## Architecture

### Crate layout

The workspace has one Rust crate:

- **`src-tauri/`** — the Tauri backend. Exposes Tauri commands to the frontend and owns all I/O, HTTP logic, and shared data types.

### Backend modules (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Tauri command definitions and the `run()` entry point |
| `git_api_handler.rs` | HTTP calls to forge APIs; parses repo URLs; dispatches by `ForgeKind` |
| `app_content_handler.rs` | CRUD on `repos.json` (read, add, delete, update status) |
| `config_handler.rs` | Read/write `config.json`; creates a default template on first run |
| `json_handler.rs` | Thin wrapper for serde_json file I/O |
| `migration.rs` | Idempotent schema migrations for `repos.json` and `config.json`; runs at every startup |

### Frontend (root)

Vanilla JS + Vite, no framework. `app.js` is the single JS file. The UI library is `oat` (bundled as `oat.min.css` / `oat.min.js`). The frontend communicates with the Tauri backend exclusively through `invoke()` calls from `@tauri-apps/api/core`.

### Data files (Linux paths)

- Repos: `~/.local/share/upstream-releases-tracker/data/repos.json`
- Config: `~/.config/upstream-releases-tracker/config.json` — endpoint metadata only; created automatically with empty values on first run. API tokens are stored in the OS keyring (not config.json).

### Tauri commands

All defined in `src-tauri/src/lib.rs`:
`get_repos`, `add_repo`, `edit_repo`, `delete_repo`, `refresh_repo`, `refresh_all`, `mark_as_updated`, `get_endpoints`, `update_endpoints`, `get_api_keys`, `update_api_keys`, `get_logs`, `clear_logs`.

### Adding a new forge

1. Add a variant to `ForgeKind` in `src-tauri/src/lib.rs`.
2. Add a branch in the `api_call` match in `src-tauri/src/git_api_handler.rs`.
3. Add a config field to `Config` in `src-tauri/src/lib.rs` and update `Config::new()`.
4. Add a migration in `migration.rs` to backfill the new config field.
5. Add the option to the `<select>` in `index.html` and the `forgeLabel` switch in `app.js`.

GitLab is implemented as of 3.4.0. Subgroup URL support (≥3 path segments) was added in the Batch 10c audit fixes.

### URL validation

`isValidRepoUrl` in `app.js` (regex pre-check for instant UX feedback) and `parse_url` in `src-tauri/src/git_api_handler.rs` (authoritative backend validation via `url::Url::parse`) must stay in sync — both enforce HTTPS URLs with at least two path segments (`host/owner/repo`). GitLab subgroup URLs with additional segments (`host/group/subgroup/project`) are valid; `parse_url` joins all segments except the last as the owner.

### Security notes

- **CSP `'unsafe-inline'` in `style-src`** (S6): required by the `oat` UI library which injects inline styles for theming. Cannot be removed without upstream support for nonce-based styles. Tauri's schema validator does not allow comments in `tauri.conf.json`, so the rationale lives here.
- **API tokens in OS keyring**: tokens are read/written only by Rust commands; JS receives token values for the Settings form and never writes them to `config.json`.
- **Forgejo trusted-host comparison** (S5): ASCII-case-insensitive only — Unicode hostnames and their punycode equivalents are treated as different entries. `validate_forgejo_host` already rejects non-ASCII input so this is not exploitable in practice.
