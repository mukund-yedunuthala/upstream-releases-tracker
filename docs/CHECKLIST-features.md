# Feature checklist: GitLab + Stronghold + Store + Settings UI

Tracks progress on the multi-feature plan from `/home/mukund/.claude/plans/plan-for-the-following-dreamy-clarke.md`.
Tick items as they land so an interrupted session can resume from this file or `git log`.

## Backend

- [x] GitLab API call implemented in `git_api_handler.rs`
- [x] `gitlab_api_key` config migration uncommented in `migration.rs`
- [x] `AppState.config` wrapped in `Mutex<Config>`; call sites updated
- [x] `tauri-plugin-stronghold`, `tauri-plugin-store`, `sha2` added to `src-tauri/Cargo.toml`
- [x] Plugins registered in `lib.rs` (stronghold with SHA-256 hash fn, store default)
- [x] Capabilities updated in `src-tauri/capabilities/default.json`
- [x] `get_vault_key` command (machine-id derived)
- [x] `update_api_keys` command (mutate in-memory config)
- [x] `update_endpoints` command (mutate in-memory config + persist)
- [x] `get_endpoints` command (read endpoints only — no keys)
- [x] `get_logs(limit)` command (read tail of tauri-plugin-log file)
- [x] Migration: scrub plaintext API key fields out of `config.json`

## Frontend

- [x] `@tauri-apps/plugin-stronghold` + `@tauri-apps/plugin-store` added to `frontend/package.json`
- [x] Settings button in topbar (`index.html`)
- [x] Settings dialog markup with API Keys / Endpoints / Logs / About tabs
- [x] CSS for tab bar + settings dialog
- [x] Startup: unlock Stronghold with vault key, hydrate AppState
- [x] Startup: load Store endpoints (fallback to backend `get_endpoints` on first run), hydrate AppState
- [x] API keys: read/write through Stronghold, then push to AppState
- [x] Endpoints: read/write through Store, then push to AppState
- [x] Log viewer wired to `get_logs`
- [x] Version display via `getVersion()`

## Verification

- [x] `cargo build` succeeds
- [x] `cargo test --workspace` passes
- [x] `npm install` in `frontend/` succeeds with the new packages
- [ ] Manual: settings round-trip — set GitHub token → close → reopen → token reused on next refresh
- [ ] Manual: GitLab public repo refresh succeeds
- [ ] Manual: config.json after first run no longer contains plaintext API keys
