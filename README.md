# Upstream Releases Tracker

A desktop app (Tauri 2.x) that tracks the latest release tags from upstream repositories and compares them against the version installed on your system.

## Supported forges

- **GitHub** (public repos and private repos with a personal access token)
- **Forgejo / Gitea** (Codeberg and self-hosted instances — see token security note below)
- **GitLab** (including subgroup project URLs)

## Development

Install frontend dependencies (only needed once or after `package.json` changes):

```sh
npm install
```

Run in dev mode (starts the Vite dev server and the Tauri shell):

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

Frontend-only dev (Vite at `localhost:1420`, no Tauri):

```sh
npm run dev
```

## Architecture

| Layer | Tech |
|---|---|
| Frontend | Vanilla JS + Vite, no framework. `app.js` is the single JS file. UI library: `oat` (bundled). |
| Backend | Tauri 2.x (`src-tauri/`). All I/O and HTTP happen in Rust. |

Tauri commands: `get_repos`, `add_repo`, `edit_repo`, `delete_repo`, `refresh_repo`, `refresh_all`, `mark_as_updated`, `get_endpoints`, `update_endpoints`, `get_api_keys`, `update_api_keys`, `get_logs`.

## Data locations (Linux)

| File | Path |
|---|---|
| Repos | `~/.local/share/upstream-releases-tracker/data/repos.json` |
| Config | `~/.config/upstream-releases-tracker/config.json` |

Config is created automatically with default endpoint and trusted-host metadata on first run.

## Token security

API tokens are stored in the OS keyring, not in `config.json`.

For Forgejo/Gitea, tokens are only sent to hosts listed in `forgejo_trusted_hosts` in `config.json` (default: `codeberg.org`). Add other trusted Forgejo instances there before adding repos from them.

## License

AGPL-3.0-or-later
