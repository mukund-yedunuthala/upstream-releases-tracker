# AGENTS.md

## Project

Upstream Releases Tracker is a Tauri 2.x desktop app for tracking latest release tags from GitHub, GitLab, and Forgejo/Gitea-compatible repositories against the locally installed version.

## Commands

- Install frontend dependencies: `pnpm install`
- Frontend-only dev server: `pnpm dev` (Vite on `localhost:1420`)
- Full Tauri dev app: `cd src-tauri && cargo tauri dev`
- Build frontend: `pnpm build`
- Build Tauri bundle: `cd src-tauri && cargo tauri build`
- Run Rust tests: `cargo test --workspace`

## Architecture

- Frontend is vanilla JS + Vite. Keep it framework-free unless explicitly asked.
- `app.js` is the single frontend JS file; `style.css` is the app stylesheet.
- The UI library is `oat`, pulled in as the `@knadh/oat` npm dependency and imported by `app.js` (`oat.min.css`, `oat.min.js`). Nothing is vendored — Vite bundles it.
- Backend calls go through Tauri `invoke()` from `@tauri-apps/api/core`. The frontend also uses `@tauri-apps/plugin-opener` (`openUrl`, for the repo-URL links on cards and the external About links) and `@tauri-apps/api/app` (`getVersion`, for the Settings panel).
- Backend lives in `src-tauri/` and owns all filesystem I/O, HTTP calls, config, migrations, theme reads, and shared data types.

## Backend Map

- `src-tauri/src/lib.rs`: Tauri commands, shared types, app state, `run()`.
- `src-tauri/src/git_api_handler.rs`: forge API HTTP calls, URL parsing, forge dispatch.
- `src-tauri/src/app_content_handler.rs`: CRUD for `repos.json`.
- `src-tauri/src/config_handler.rs`: read/write `config.json`, default config creation.
- `src-tauri/src/json_handler.rs`: serde_json file I/O wrapper.
- `src-tauri/src/migration.rs`: idempotent startup migrations for repo/config data.
- `src-tauri/src/theme_handler.rs`: system theme/accent lookup and `theme_mode` persistence via the `system-theme` crate.

## Data

Linux paths:

- Repos: `~/.local/share/upstream-releases-tracker/data/repos.json`
- Config: `~/.config/upstream-releases-tracker/config.json`, created automatically with empty values on first run.

API tokens live in the OS keyring. Do not persist token values to `config.json`; endpoint metadata, Forgejo trusted-host metadata, and `theme_mode` belong there.

## Tauri Commands

Defined in `src-tauri/src/lib.rs`:

`get_repos`, `add_repo`, `edit_repo`, `delete_repo`, `refresh_repo`, `refresh_all`, `mark_as_updated`, `get_endpoints`, `update_endpoints`, `get_api_keys`, `update_api_keys`, `get_theme_settings`, `update_theme_settings`, `get_logs`, `clear_logs`.

## Adding a Forge

1. Add a variant to `ForgeKind` in `src-tauri/src/lib.rs`.
2. Add a branch to the `api_call` match in `src-tauri/src/git_api_handler.rs`.
3. Add a config field to `Config` in `src-tauri/src/lib.rs` and update `Config::new()`.
4. Add a migration in `src-tauri/src/migration.rs` to backfill the new config field.
5. Add the option to **both** forge selects in `index.html` (`#repo-host-select` and `#edit-host-select`). Option values are PascalCase and must match the `ForgeKind` variant name (e.g. `ForgejoCompatible`).
6. Add a `case` to the `forgeLabel` switch in `app.js`.

## Development Notes

- Keep `isValidRepoUrl` in `app.js` and `parse_url` in `src-tauri/src/git_api_handler.rs` behaviorally aligned. Both require HTTPS and at least two path segments (`host/owner/repo`), with GitLab subgroup URLs (`host/group/subgroup/project`) valid; `parse_url` joins all segments except the last as the owner. Known divergence: `parse_url` caps URLs at 2048 characters, `isValidRepoUrl` has no length cap. Fix the JS side if you touch this.
- Prefer small, direct changes. Do not add frontend frameworks, new build tools, or broad abstractions without a concrete need.

## Security Notes

- **CSP `'unsafe-inline'` in `style-src`** (S6): required by the `oat` UI library, which injects inline styles for theming. Cannot be removed without upstream support for nonce-based styles. Tauri's schema validator does not allow comments in `tauri.conf.json`, so this rationale lives here. Preserve it unless `oat` no longer needs injected inline styles.
- **API tokens in the OS keyring**: tokens are read/written only by Rust commands. The frontend receives token values for the Settings form but never writes them to `config.json`.
- **Forgejo trusted hosts**: Forgejo/Gitea tokens must only be sent to hosts listed in `forgejo_trusted_hosts`. Host comparison is ASCII-case-insensitive only, so Unicode hostnames and their punycode equivalents are treated as distinct entries. `validate_forgejo_host` rejects non-ASCII input, so this is not exploitable in practice.
