# BUGS.md Fix Checklist

Live tracking artifact for the 9-batch fix plan derived from `BUGS.md`.
The plan itself lives at `~/.claude/plans/read-bugs-md-make-a-ticklish-reddy.md`.

## How to resume after an interruption

A coding agent picking this up cold should:

1. **Read this file top to bottom.** The first unchecked `[ ]` item in the first batch with an unchecked Commit box is the next thing to do.
2. **Cross-check against git.** Run `git log --oneline release..HEAD` (the working branch is `release`). Each batch lands one commit; the commit subject is listed under each batch. If a commit exists for a batch where the Commit box is unchecked, the checklist is stale — tick it and continue.
3. **Cross-check against the working tree.** Run `git status`. If there are uncommitted changes, the previous run was interrupted mid-batch. Inspect the diff, finish the remaining unchecked items in that batch, commit, then continue.
4. **Update this file as you go.** Tick each `[x]` the moment its change lands in the working tree. Tick the Commit box only after `git commit` succeeds. Push the checklist update inside the same commit as the batch work — it travels with the change.

## Conventions

- `[ ]` open, `[x]` done.
- Each batch ends with a **Commit** checkbox plus a suggested commit subject. Keep batches as single commits per the user's preference (grouped commits per batch).
- Numbered items map 1:1 to `BUGS.md` issues (e.g. `#3` = BUGS.md issue #3).
- File paths and rationale are in the plan file; this checklist is intentionally terse.

---

## Batch 1 — Edit-flow data loss & dead frontend state

- [x] **#3** `frontend/app.js:103` — `data.host` → `data.host_kind`
- [x] **#2** `frontend/app.js:281-282` — `add_repo` keys: `host_url` → `host`, `host_kind` → `forge`
- [x] **#13** `frontend/app.js:3,5,250,259,289` — delete `editingHostKind` state
- [x] **#20** `frontend/app.js:33-35` — delete dead `btoa(url)` id
- [ ] Smoke test: edit existing repo → save unchanged → repo still present
- [x] **Commit** — `fix(frontend): repair edit dialog (BUGS #2 #3 #13 #20)`

---

## Batch 2 — Config & dependency hygiene

- [x] **#6** `src-tauri/tauri.conf.json:14` — `withGlobalTauri: false`
- [x] **#11** `src-tauri/src/git_api_handler.rs:8-15` — `.timeout(15s)`, `.connect_timeout(5s)`
- [x] **#16** `src-tauri/Cargo.toml:29` — drop `"blocking"` from reqwest features
- [x] **#26** `Cargo.toml:15-16` — add `resolver = "2"` to `[workspace]`
- [x] **#22** `frontend/style.css:298-318` — delete duplicate mobile block
- [x] **#29** `frontend/style.css:142` — `1.125rem` → `var(--font-title)` (BUGS.md said `--font-body` but that's the wrong token; 1.125rem matches `--font-title`'s base value)
- [x] **#23** `frontend/index.html:26-30,66-70` — `maxlength="2048"` on both URL inputs
- [x] **#24** `frontend/index.html:50,74-81` — drop experimental `closedby`/`commandfor`/`command`, wire JS Cancel handler
- [ ] Smoke test: `window.__TAURI__` undefined in DevTools; hung host times out ~15s
- [x] **Commit** — `chore: config & dependency hygiene (BUGS #6 #11 #16 #22 #23 #24 #26 #29)`

---

## Batch 3 — Backend cleanup & silent-failure fixes

- [x] **#9** Delete `src/main.rs`, strip root `[package]`/`[dependencies]` in `Cargo.toml`, drop WASM deps
- [x] **#9** Confirm `cargo build --workspace` green
- [x] **#30** Inline `helper.rs::read_config` into `src-tauri/src/lib.rs`, delete file, drop `mod helper;`
- [x] **#25** `src-tauri/src/git_api_handler.rs:160,185` — `unwrap_or("")` → `Err("API response missing tag_name")`
- [x] **#18** `src-tauri/src/app_content_handler.rs:64-85` — mutate in-place, single write (also removed dead `upd_repo`)
- [x] **#17** `src-tauri/src/json_handler.rs:31` — produce `repos.json.tmp` not `repos.tmp`
- [x] Smoke test: `cargo test --workspace` green
- [x] **Commit** — `refactor(backend): cleanup & tighten IO (BUGS #9 #17 #18 #25 #30)` (Note: public/repos.json was accidentally committed via git add -A; will be removed from tracking in Batch 9 #33)

---

## Batch 4 — URL parsing & forge-kind correctness

- [x] **#12** Add `url` to `src-tauri/Cargo.toml`; rewrite `parse_url` with `url::Url::parse()`, reject non-https, support nested paths, cap length 2048
- [x] **#28** Update JS comment to point at `parse_url` in `git_api_handler.rs` (kept JS regex for sync UX — avoids unnecessary async round-trip)
- [x] **#28** Update CLAUDE.md "URL validation" section; also fixed stale `get_config` in Tauri commands list
- [x] **#15** Remove `ForgeKind::Unknown` variant from `tracker_libs/src/lib.rs`; drop the dead branch in `git_api_handler.rs`
- [x] **#15** Migration 5 in `migration.rs` coerces persisted `Unknown` → `GitHub`
- [ ] Smoke test: `file:///etc/passwd` rejected; nested GitLab path parses
- [x] **Commit** — `fix(forge): tighten URL parsing & remove Unknown variant (BUGS #12 #15 #28)`

---

## Batch 5 — XSS hardening

- [x] **#1** `frontend/app.js:50-88` — rebuilt `buildCard` with static innerHTML skeleton + DOM textContent for all API/user data; href only set for https: URLs
- [x] **#5** `src-tauri/tauri.conf.json:23` — strict CSP `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: https://ipc.localhost`
- [ ] **#5** Verify dev (Vite HMR) and prod bundle both load
- [x] **#7** Remove `get_config` command from `src-tauri/src/lib.rs` and `generate_handler![]`
- [ ] Smoke test: inject `<img src=x onerror=alert(1)>` as tag → renders literally; `invoke('get_config')` errors
- [x] **Commit** — `security: XSS hardening & redact config (BUGS #1 #5 #7)`

---

## Batch 6 — Forgejo host trust boundary

- [x] **#4** Add `forgejo_trusted_hosts: Vec<String>` to `Config` in `tracker_libs/src/lib.rs`; default `vec!["codeberg.org".into()]`
- [x] **#4** Backfill via `src-tauri/src/migration.rs` (config migration 2)
- [x] **#4** Gate `forgejo_api_call` on allowlist; error message tells user to add host to config.json
- [x] **#4** Error propagates to frontend as a toast via existing error handling (no separate wiring needed)
- [ ] Smoke test: `evil.example.com` Forgejo URL rejected; `codeberg.org` works
- [x] **Commit** — `security: Forgejo host allowlist (BUGS #4)`

---

## Batch 7 — Concurrency & batched refresh

- [x] **#14** Frontend now reports `"N refreshed, M failed"` aggregate toast from refresh_all result
- [x] **#10** `refresh_all` Tauri command: load once, parallel `tokio::task::spawn`, re-read + write once under lock
- [x] **#10** Frontend uses `invoke("refresh_all")` instead of JS loop
- [x] **#8** All RMW commands serialized through `DataFileState.lock: tokio::sync::Mutex<()>`; HTTP calls happen outside the lock
- [x] **#8** Edit delete+add still two separate calls from frontend (transactional wrap deferred — needs its own design)
- [ ] Smoke test: spam Refresh All during a single Refresh → no lost writes; 9/10 success path reports counts
- [x] **Commit** — `perf+correctness: batched refresh & data-file mutex (BUGS #8 #10 #14)`

---

## Batch 8 — Observability & card-rendering refactor

- [x] **#19** Add `tauri-plugin-log` wired in `run()`; `log = "^0.4"` as direct dep
- [x] **#19** Convert `eprintln!` in `src-tauri/src/migration.rs` to `log::info!` / `log::warn!` / `log::error!`
- [x] **#21** Single delegated `click` handler on `#repo-grid` reading `data-action` + `closest('[data-url]')`; per-card listeners removed; host_kind stored in `card.dataset.hostKind`
- [x] **#21** `openEditDialog` signature simplified (removed unused `name` param)
- [ ] Smoke test: corrupt config → see error in log file; with 20 repos only one listener on the grid
- [x] **Commit** — `chore: logging plugin & card render delegation (BUGS #19 #21)`

---

## Batch 9 — Cleanup (README last)

- [x] **#27** Built frontend — bundle is 12.79 kB; grep confirms zero `@tauri-apps/api` non-core exports (tree-shaking already working)
- [x] **#32** `tracker_libs/src/lib.rs` — added comment on intentional URL/RepoData denormalization
- [x] **#33** Added `frontend/dist/` and `public/repos.json` to `.gitignore`; untracked `public/repos.json` with `git rm --cached`
- [x] **#31** README rewritten: app description, dev/build commands, supported forges, data locations, token security note
- [ ] **Commit** — `chore: gitignore, comments & README overhaul (BUGS #27 #31 #32 #33)`

---

## Final verification

- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` green
- [ ] `cd src-tauri && cargo tauri dev` — app launches, full flow exercised: add, refresh, refresh-all, edit, mark-updated, delete
- [ ] `cd src-tauri && cargo tauri build` succeeds
- [ ] `git log --oneline release..HEAD` shows 9 commits, one per batch
