# AGENTS.md

## Project

Upstream Releases Tracker is a Tauri 2.x desktop app for tracking latest release tags from GitHub, GitLab, and Forgejo/Gitea-compatible repositories against the locally installed version.

## Commands

- Install frontend dependencies: `npm install`
- Frontend-only dev server: `npm run dev` (Vite on `localhost:1420`)
- Full Tauri dev app: `cd src-tauri && cargo tauri dev`
- Build frontend: `npm run build`
- Build Tauri bundle: `cd src-tauri && cargo tauri build`
- Run Rust tests: `cargo test --workspace`

## Architecture

- Frontend is vanilla JS + Vite. Keep it framework-free unless explicitly asked.
- `app.js` is the single frontend JS file.
- UI uses bundled `oat` assets.
- Frontend talks to Rust only through Tauri `invoke()` calls from `@tauri-apps/api/core`.
- Backend lives in `src-tauri/` and owns all filesystem I/O, HTTP calls, config, migrations, and shared data types.

## Backend Map

- `src-tauri/src/lib.rs`: Tauri commands, shared types, app state, `run()`.
- `src-tauri/src/git_api_handler.rs`: forge API HTTP calls, URL parsing, forge dispatch.
- `src-tauri/src/app_content_handler.rs`: CRUD for `repos.json`.
- `src-tauri/src/config_handler.rs`: read/write `config.json`, default config creation.
- `src-tauri/src/json_handler.rs`: serde_json file I/O wrapper.
- `src-tauri/src/migration.rs`: idempotent startup migrations for repo/config data.

## Data

Linux paths:

- Repos: `~/.local/share/upstream-releases-tracker/data/repos.json`
- Config: `~/.config/upstream-releases-tracker/config.json`

API tokens live in the OS keyring. Do not persist token values to `config.json`; endpoint metadata and Forgejo trusted-host metadata belong there.

## Tauri Commands

Defined in `src-tauri/src/lib.rs`:

`get_repos`, `add_repo`, `edit_repo`, `delete_repo`, `refresh_repo`, `refresh_all`, `mark_as_updated`, `get_endpoints`, `update_endpoints`, `get_api_keys`, `update_api_keys`, `get_logs`.

## Development Notes

- Keep `isValidRepoUrl` in `app.js` and `parse_url` in `src-tauri/src/git_api_handler.rs` behaviorally aligned.
- URL validation should require HTTPS and at least `host/owner/repo`; GitLab subgroup URLs may have additional path segments.
- When adding a forge, update `ForgeKind`, API dispatch, config defaults/migrations, the HTML forge selector, and frontend labels.
- Preserve the CSP `style-src 'unsafe-inline'` rationale unless `oat` no longer needs injected inline styles.
- Forgejo/Gitea tokens must only be sent to hosts listed in `forgejo_trusted_hosts`.
- Prefer small, direct changes. Do not add frontend frameworks, new build tools, or broad abstractions without a concrete need.
