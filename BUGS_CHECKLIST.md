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
- [ ] **Commit** — `refactor(backend): cleanup & tighten IO (BUGS #9 #17 #18 #25 #30)`

---

## Batch 4 — URL parsing & forge-kind correctness

- [ ] **#12** Add `url` to `src-tauri/Cargo.toml`; rewrite `parse_url` with `url::Url::parse()`, reject non-https, support nested paths, cap length 2048
- [ ] **#28** Implement `validate_repo_url` Tauri command in Rust; call from `frontend/app.js` via `invoke`; delete JS regex at `frontend/app.js:294-298`
- [ ] **#28** Update CLAUDE.md "URL validation" section
- [ ] **#15** Remove `ForgeKind::Unknown` variant from `tracker_libs/src/lib.rs`; drop the dead branch in `git_api_handler.rs:55-58`
- [ ] **#15** Add migration in `migration.rs` to coerce any persisted `Unknown` (decide policy when implementing)
- [ ] Smoke test: `file:///etc/passwd` rejected; nested GitLab path parses
- [ ] **Commit** — `fix(forge): tighten URL parsing & remove Unknown variant (BUGS #12 #15 #28)`

---

## Batch 5 — XSS hardening

- [ ] **#1** `frontend/app.js:50-88` — rebuild `buildCard` via `createElement` + `textContent`; validate href scheme
- [ ] **#5** `src-tauri/tauri.conf.json:23` — strict CSP `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: https://ipc.localhost`
- [ ] **#5** Verify dev (Vite HMR) and prod bundle both load
- [ ] **#7** Remove `get_config` command from `src-tauri/src/lib.rs` and `generate_handler![]`
- [ ] Smoke test: inject `<img src=x onerror=alert(1)>` as tag → renders literally; `invoke('get_config')` errors
- [ ] **Commit** — `security: XSS hardening & redact config (BUGS #1 #5 #7)`

---

## Batch 6 — Forgejo host trust boundary

- [ ] **#4** Add `forgejo_hosts: Vec<String>` to `Config` in `tracker_libs/src/lib.rs`; default `vec!["codeberg.org".into()]`
- [ ] **#4** Backfill via `src-tauri/src/migration.rs`
- [ ] **#4** Gate `forgejo_api_call` on allowlist membership; clear error if not listed
- [ ] **#4** Frontend toast wording for host-not-trusted errors
- [ ] Smoke test: `evil.example.com` Forgejo URL rejected; `codeberg.org` works
- [ ] **Commit** — `security: Forgejo host allowlist (BUGS #4)`

---

## Batch 7 — Concurrency & batched refresh

- [ ] **#14** Per-item try/catch in current loop, aggregate `{successes, failures}` reporting (only if landing standalone before #10)
- [ ] **#10** Add `refresh_all` Tauri command — load once, `tokio::join_all`, write once; return `{ok, err}`
- [ ] **#10** Frontend uses the batch command instead of the JS loop
- [ ] **#8** Wrap data file/state in `tauri::State<Arc<tokio::sync::Mutex<...>>>`; serialize all RMW commands through it
- [ ] **#8** Wrap edit (delete+add) transactionally in a single backend command (closes the residual gap from Batch 1 #2)
- [ ] Smoke test: spam Refresh All during a single Refresh → no lost writes; 9/10 success path reports counts
- [ ] **Commit** — `perf+correctness: batched refresh & data-file mutex (BUGS #8 #10 #14)`

---

## Batch 8 — Observability & card-rendering refactor

- [ ] **#19** Add `tauri-plugin-log` (or `tracing` + file appender) wired in `run()`
- [ ] **#19** Convert `eprintln!` in `src-tauri/src/migration.rs:54,56,60,92,96` to `log::warn!` / `log::error!`
- [ ] **#21** Extract card template to `DocumentFragment` factory in `frontend/app.js`
- [ ] **#21** Single delegated `click` handler on `#repo-grid` reading `data-action` + `closest('[data-url]')`
- [ ] Smoke test: corrupt config → see error in log file; with 20 repos only one listener on the grid
- [ ] **Commit** — `chore: logging plugin & card render delegation (BUGS #19 #21)`

---

## Batch 9 — Cleanup (README last)

- [ ] **#27** `cd frontend && npm run build`; inspect `dist/assets/*.js` for `@tauri-apps/api` non-core exports
- [ ] **#32** `tracker_libs/src/lib.rs:38-46` — comment on intentional `BTreeMap` key / `RepoData` denormalization
- [ ] **#33** Confirm `.gitignore` covers `frontend/dist/`, `dist/`; add `/public/repos.json` (dev-local data)
- [ ] **#33** Untrack `public/repos.json` if currently tracked
- [ ] **#31** Rewrite `README.md` — app description, dev/build commands (mirror CLAUDE.md), supported forges, config/data locations, token-storage security note
- [ ] **Commit** — `chore: gitignore, comments & README overhaul (BUGS #27 #31 #32 #33)`

---

## Final verification

- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` green
- [ ] `cd src-tauri && cargo tauri dev` — app launches, full flow exercised: add, refresh, refresh-all, edit, mark-updated, delete
- [ ] `cd src-tauri && cargo tauri build` succeeds
- [ ] `git log --oneline release..HEAD` shows 9 commits, one per batch
