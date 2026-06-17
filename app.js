import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import "@knadh/oat/oat.min.css";
import "@knadh/oat/oat.min.js";
// State
let editingUrl = null;
let deletingUrl = null;

function forgejoVaultKey(host) {
  return `forgejo_token:${host}`;
}
const ENDPOINT_DEFAULTS = {
  github: "https://api.github.com/repos/",
  gitlab: "https://gitlab.com/api/v4/projects/",
  // Default host list uses the ForgejoHost object format.
  forgejoHosts: [{ host: "codeberg.org", token_ref: "forgejo_token:codeberg.org" }],
};

const settingsState = {
  keys: { github: "", gitlab: "" },
  forgejoTokens: [],
  endpoints: {
    github: ENDPOINT_DEFAULTS.github,
    gitlab: ENDPOINT_DEFAULTS.gitlab,
    forgejoHosts: [...ENDPOINT_DEFAULTS.forgejoHosts],
  },
};

// DOM refs
const repoGrid = document.getElementById("repo-grid");
const urlInput = document.getElementById("repo-url-input");
const repoAddField = document.getElementById("repo-add-field");
const addBtn = document.getElementById("add-repo-btn");
const refreshAllBtn = document.getElementById("refresh-all-btn");

function setRepoUrlError(message) {
  if (message) {
    repoAddField.setAttribute("data-field", "error");
    urlInput.setAttribute("aria-invalid", "true");
    document.getElementById("repo-url-error").textContent = message;
  } else {
    repoAddField.setAttribute("data-field", "");
    urlInput.removeAttribute("aria-invalid");
  }
}
const editDialog = document.getElementById("edit-dialog");
const editUrlInput = document.getElementById("edit-url-input");
const hostSelect = document.getElementById("repo-host-select");
const editHostSelect = document.getElementById("edit-host-select");
const editCancelBtn = document.getElementById("edit-cancel-btn");

editCancelBtn.addEventListener("click", () => editDialog.close());

// Delegated handler for all card action buttons — one listener for all cards.
repoGrid.addEventListener("keydown", (e) => {
  if (e.key !== "Enter" && e.key !== " ") return;
  const copyEl = e.target.closest("[data-copy]");
  if (copyEl && copyEl.textContent && copyEl.textContent !== "—") {
    e.preventDefault();
    navigator.clipboard.writeText(copyEl.textContent).then(() => {
      copyEl.classList.add("copied");
      setTimeout(() => copyEl.classList.remove("copied"), 1500);
    });
  }
});

repoGrid.addEventListener("click", (e) => {
  const copyEl = e.target.closest("[data-copy]");
  if (copyEl && copyEl.textContent && copyEl.textContent !== "—") {
    navigator.clipboard.writeText(copyEl.textContent).then(() => {
      copyEl.classList.add("copied");
      setTimeout(() => copyEl.classList.remove("copied"), 1500);
    });
    return;
  }

  const btn = e.target.closest("[data-action]");
  if (!btn) return;
  const card = btn.closest("[data-url]");
  if (!card) return;
  const url = card.dataset.url;
  switch (btn.dataset.action) {
    case "refresh":
      handleRefresh(btn, url);
      break;
    case "mark-updated":
      handleMarkAsUpdated(url);
      break;
    case "edit":
      openEditDialog(url, card.dataset.hostKind);
      break;
    case "delete":
      openDeleteDialog(url);
      break;
  }
});

document.addEventListener("click", async (e) => {
  const anchor = e.target.closest("a[href]");
  if (!anchor) return;

  const href = anchor.getAttribute("href");
  if (!href || href.startsWith("#")) return;

  e.preventDefault();
  try {
    await openUrl(href);
  } catch (err) {
    ot.toast(String(err), "Failed to open link", { variant: "danger" });
  }
});

function forgeLabel(hostKind) {
  switch (hostKind) {
    case "GitHub":
      return "GitHub";
    case "GitLab":
      return "GitLab";
    case "ForgejoCompatible":
      return "Codeberg / Forgejo";
    default:
      return hostKind || "Unknown";
  }
}

function buildCard(url, data) {
  const isUpToDate =
    !!data.system_version && data.system_version === data.latest_release;
  const badgeVariant = isUpToDate ? "success" : "warning";
  const badgeText = isUpToDate ? "Up to date" : "Update available";
  // Only allow https: URLs in the href to prevent javascript: injection
  const safeUrl = url.startsWith("https://") ? url : null;
  const shortUrl = safeUrl ? safeUrl.replace(/^https:\/\//, "") : "";
  const name =
    data.owner && data.repo_name
      ? `${data.owner}/${data.repo_name}`
      : shortUrl;
  const latest = data.latest_release || "—";
  const system = data.system_version || "—";
  const card = document.createElement("article");
  card.className = "card repo-card";
  card.dataset.url = url;

  // Static structural markup — no user/API data interpolated here
  card.innerHTML = `
    <header>
      <div class="repo-card-title">
        <strong class="js-card-name"></strong>
        <span class="js-card-badge"></span>
      </div>
      <div class="repo-card-subtitle">
        <span class="js-card-host-badge badge outline"></span>
        <a class="repo-url js-card-url"></a>
      </div>
    </header>
    <dl class="repo-card-meta">
      <dt>Latest release</dt>
      <dd><code class="js-card-latest" data-copy data-tooltip="Click to copy" tabindex="0" role="button"></code></dd>
      <dt>Installed version</dt>
      <dd><code class="js-card-system" data-copy data-tooltip="Click to copy" tabindex="0" role="button"></code></dd>
    </dl>
    <details class="repo-card-notes">
      <summary>Release notes</summary>
      <pre><code class="js-card-notes"></code></pre>
    </details>
    <footer class="repo-card-actions">
      <button class="small outline" data-action="refresh">Refresh</button>
      <button class="small outline" data-action="mark-updated">Mark as updated</button>
      <button class="small outline" data-action="edit">Edit</button>
      <button class="small outline" data-variant="danger" data-action="delete">Delete</button>
    </footer>
  `;

  // Inject all dynamic data via DOM — never via innerHTML interpolation
  card.querySelector(".js-card-name").textContent = name;
  const badge = card.querySelector(".js-card-badge");
  badge.className = "badge";
  badge.dataset.variant = badgeVariant;
  badge.textContent = badgeText;
  const hostBadge = card.querySelector(".js-card-host-badge");
  hostBadge.textContent = forgeLabel(data.host_kind);
  const link = card.querySelector(".js-card-url");
  if (safeUrl) link.href = safeUrl;
  link.textContent = shortUrl;
  card.querySelector(".js-card-latest").textContent = latest;
  card.querySelector(".js-card-system").textContent = system;

  const notes = data.release_notes || "";
  const notesEl = card.querySelector(".repo-card-notes");
  if (notes) {
    card.querySelector(".js-card-notes").textContent = notes;
  } else {
    notesEl.dataset.empty = "";
    notesEl.querySelector("summary").tabIndex = -1;
  }

  if (isUpToDate) {
    card.querySelector("[data-action='mark-updated']").hidden = true;
  }

  // Store host_kind for the delegated edit handler (see repoGrid click listener)
  card.dataset.hostKind = data.host_kind || "";

  return card;
}

// Handle update
async function handleMarkAsUpdated(url) {
  try {
    await invoke("mark_as_updated", { url });
    ot.toast("Marked as updated", "Done", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Mark as updated failed", { variant: "danger" });
  }
}

// Toolbar + status footer
const repoToolbar = document.getElementById("repo-toolbar");
const repoSearch = document.getElementById("repo-search");
const repoFilter = document.getElementById("repo-filter");
const repoSort = document.getElementById("repo-sort");
const appStatus = document.getElementById("app-status");
const appStatusCounts = document.getElementById("app-status-counts");
const appStatusRefresh = document.getElementById("app-status-refresh");

let allRepos = [];
let viewState = { search: "", filter: "all", sort: "name" };
let lastRefreshAt = null;

function isRepoUpToDate({ data }) {
  return !!data.system_version && data.system_version === data.latest_release;
}

function repoSortKey({ url, data }) {
  const name =
    data.owner && data.repo_name
      ? `${data.owner}/${data.repo_name}`.toLowerCase()
      : url.toLowerCase();
  return name;
}

function applyView() {
  const q = viewState.search.toLowerCase();
  let rows = allRepos.filter(({ url, data }) => {
    if (viewState.filter === "outdated" && isRepoUpToDate({ data })) return false;
    if (viewState.filter === "uptodate" && !isRepoUpToDate({ data })) return false;
    if (!q) return true;
    const haystack = [
      url,
      data.owner,
      data.repo_name,
      data.host_kind,
      data.latest_release,
      data.system_version,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return haystack.includes(q);
  });
  rows.sort((a, b) => {
    if (viewState.sort === "host") {
      return (a.data.host_kind || "").localeCompare(b.data.host_kind || "");
    }
    if (viewState.sort === "status") {
      return Number(isRepoUpToDate(a)) - Number(isRepoUpToDate(b));
    }
    return repoSortKey(a).localeCompare(repoSortKey(b));
  });
  return rows;
}

function renderEmpty() {
  repoGrid.innerHTML = "";
  const empty = document.createElement("article");
  empty.className = "card repo-empty";
  empty.innerHTML = `
    <h4>No repositories tracked yet</h4>
    <p class="text-light">Add one using the input above.</p>
  `;
  repoGrid.appendChild(empty);
}

function renderNoMatches() {
  repoGrid.innerHTML = "";
  const empty = document.createElement("article");
  empty.className = "card repo-empty";
  empty.innerHTML = `
    <h4>No matching repositories</h4>
    <p class="text-light">Try a different search or filter.</p>
  `;
  repoGrid.appendChild(empty);
}

function renderSkeletons() {
  repoGrid.innerHTML = "";
  for (let i = 0; i < 3; i++) {
    const card = document.createElement("article");
    card.className = "card repo-card repo-skeleton";
    card.setAttribute("aria-hidden", "true");
    card.innerHTML = `
      <div role="status" class="skeleton line" style="width: 40%"></div>
      <div role="status" class="skeleton line"></div>
      <div role="status" class="skeleton line" style="width: 60%"></div>
    `;
    repoGrid.appendChild(card);
  }
}

function renderRepos() {
  if (allRepos.length === 0) {
    repoToolbar.hidden = true;
    appStatus.hidden = true;
    renderEmpty();
    return;
  }
  repoToolbar.hidden = false;
  appStatus.hidden = false;
  const rows = applyView();
  if (rows.length === 0) {
    renderNoMatches();
  } else {
    repoGrid.innerHTML = "";
    rows.forEach(({ url, data }) => {
      repoGrid.appendChild(buildCard(url, data));
    });
  }
  updateStatus();
}

function updateStatus() {
  const total = allRepos.length;
  const outdated = allRepos.filter((r) => !isRepoUpToDate(r)).length;
  appStatusCounts.textContent = `${total} repo${total === 1 ? "" : "s"} · ${outdated} outdated`;
  appStatusRefresh.textContent = lastRefreshAt
    ? `Last refresh: ${lastRefreshAt.toLocaleTimeString()}`
    : "";
}

// Load all repos
async function loadRepos({ showSkeletons = false } = {}) {
  if (showSkeletons) renderSkeletons();
  try {
    const repos = await invoke("get_repos");
    allRepos = Object.entries(repos).map(([url, data]) => ({ url, data }));
    renderRepos();
  } catch (e) {
    ot.toast(String(e), "Failed to load repos", { variant: "danger" });
  }
}

repoSearch.addEventListener("input", () => {
  viewState.search = repoSearch.value.trim();
  renderRepos();
});

repoFilter.addEventListener("click", (e) => {
  const btn = e.target.closest("button[data-filter]");
  if (!btn) return;
  viewState.filter = btn.dataset.filter;
  repoFilter.querySelectorAll("button[data-filter]").forEach((b) => {
    const active = b === btn;
    b.setAttribute("aria-pressed", active ? "true" : "false");
    b.classList.toggle("outline", !active);
  });
  renderRepos();
});

repoSort.addEventListener("change", () => {
  viewState.sort = repoSort.value;
  renderRepos();
});

// Add repo
addBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  const forge = hostSelect.value;

  if (!url) {
    setRepoUrlError("Please enter a repository URL");
    urlInput.focus();
    return;
  }
  if (!isValidRepoUrl(url)) {
    setRepoUrlError("URL must be https://host/owner/repo");
    urlInput.focus();
    return;
  }

  addBtn.disabled = true;
  addBtn.setAttribute("aria-busy", "true");
  try {
    await invoke("add_repo", { url, forge });
    ot.toast("Repository added", "Done", { variant: "success" });
    urlInput.value = "";
    setRepoUrlError(null);
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Add repo failed", { variant: "danger" });
  } finally {
    addBtn.disabled = false;
    addBtn.removeAttribute("aria-busy");
  }
});

urlInput.addEventListener("input", () => {
  if (repoAddField.getAttribute("data-field") === "error") {
    setRepoUrlError(null);
  }
});


// Enter key on input
urlInput.addEventListener("keydown", (ev) => {
  if (ev.key === "Enter") {
    ev.preventDefault();
    addBtn.click();
  }
});

// Refresh single repo
async function handleRefresh(btn, url) {
  btn.disabled = true;
  btn.setAttribute("aria-busy", "true");
  try {
    await invoke("refresh_repo", { url });
    ot.toast("Repo refreshed", "Done", { variant: "success" });
    lastRefreshAt = new Date();
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Refresh failed", { variant: "danger" });
  } finally {
    btn.disabled = false;
    btn.removeAttribute("aria-busy");
  }
}

// Refresh all repos
refreshAllBtn.addEventListener("click", async () => {
  refreshAllBtn.disabled = true;
  refreshAllBtn.setAttribute("aria-busy", "true");
  try {
    const { ok, err } = await invoke("refresh_all");
    if (err.length === 0) {
      ot.toast(`All ${ok.length} repos refreshed`, "Done", {
        variant: "success",
      });
    } else {
      ot.toast(
        `${ok.length} refreshed, ${err.length} failed:\n${err.map(([u, e]) => `${u}: ${e}`).join("\n")}`,
        "Partial refresh",
        { variant: "warning" },
      );
    }
    lastRefreshAt = new Date();
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Refresh all failed", { variant: "danger" });
  } finally {
    refreshAllBtn.disabled = false;
    refreshAllBtn.removeAttribute("aria-busy");
  }
});

// Delete repo
async function handleDelete(url) {
  try {
    await invoke("delete_repo", { url });
    ot.toast("Repository deleted", "", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Delete failed", { variant: "danger" });
  }
}

// Delete confirmation dialog
const deleteDialog = document.getElementById("delete-dialog");
const deleteCancelBtn = document.getElementById("delete-cancel-btn");

deleteCancelBtn.addEventListener("click", () => deleteDialog.close());

function openDeleteDialog(url) {
  deletingUrl = url;
  deleteDialog.showModal();
}

deleteDialog.addEventListener("close", async () => {
  // Capture deletingUrl immediately so a concurrent openDeleteDialog() call
  // cannot clobber it before the await below completes (mirrors editDialog).
  const oldUrl = deletingUrl;
  deletingUrl = null;
  if (deleteDialog.returnValue !== "confirm" || !oldUrl) return;
  await handleDelete(oldUrl);
});

// Edit dialog open
function openEditDialog(url, hostKind) {
  editingUrl = url;
  editUrlInput.value = url;
  editHostSelect.value = hostKind;
  editDialog.showModal();
}
// Edit dialog
editDialog.addEventListener("close", async () => {
  // Capture editingUrl immediately so a concurrent openEditDialog() call
  // cannot clobber it before the await below completes (#42).
  const oldUrl = editingUrl;
  editingUrl = null;

  if (editDialog.returnValue !== "save") {
    return;
  }

  const newUrl = editUrlInput.value.trim();
  const newHostKind = editHostSelect.value;

  if (!oldUrl || !newUrl) {
    return;
  }
  if (!isValidRepoUrl(newUrl)) {
    ot.toast("URL must be https://host/owner/repo", "Invalid URL", {
      variant: "danger",
    });
    return;
  }

  try {
    await invoke("edit_repo", { oldUrl, newUrl, forge: newHostKind });
    ot.toast("Repository updated", "Saved", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Update failed", { variant: "danger" });
  }
});

// Mirrors parse_url in git_api_handler.rs — must stay in sync with Rust validation.
// Accepts 2+ non-empty path segments so GitLab subgroup URLs work
// (e.g. https://gitlab.com/group/subgroup/project).
function isValidRepoUrl(url) {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:") return false;
    const segments = parsed.pathname.split("/").filter(Boolean);
    return segments.length >= 2;
  } catch {
    return false;
  }
}

// ── Settings dialog ────────────────────────────────────────────────────────
const settingsDialog = document.getElementById("settings-dialog");
const settingsBtn = document.getElementById("settings-btn");
const settingsSaveBtn = document.getElementById("settings-save-btn");
const settingsShowKeys = document.getElementById("settings-show-keys");
const settingsLogsRefresh = document.getElementById("settings-logs-refresh");
const settingsLogsOutput = document.getElementById("settings-logs-output");
const settingsLogsStatus = document.getElementById("settings-logs-status");
const settingsAppVersion = document.getElementById("settings-app-version");

const settingsInputs = {
  github: document.getElementById("settings-github-key"),
  gitlab: document.getElementById("settings-gitlab-key"),
  githubEndpoint: document.getElementById("settings-github-endpoint"),
  gitlabEndpoint: document.getElementById("settings-gitlab-endpoint"),
};
const forgejoHostList = document.getElementById("settings-forgejo-host-list");
const forgejoAddHostBtn = document.getElementById("settings-forgejo-add-host");

async function initApiKeys() {
  const keys = await invoke("get_api_keys");
  settingsState.keys = {
    github: keys.github_api_key || "",
    gitlab: keys.gitlab_api_key || "",
  };
  settingsState.forgejoTokens = keys.forgejo_tokens || [];
}

/// Normalises the forgejo_trusted_hosts value from the backend.
/// Accepts both old Vec<String> format and new Vec<{host, token_ref}> format,
/// always returning the new object format.
function normaliseForgejoHosts(raw) {
  if (!Array.isArray(raw) || raw.length === 0) return [...ENDPOINT_DEFAULTS.forgejoHosts];
  return raw.map((entry) => {
    if (typeof entry === "string") {
      // Old format — upgrade to object in place.
      return { host: entry, token_ref: forgejoVaultKey(entry) };
    }
    return entry;
  });
}

async function initEndpoints() {
  const backend = await invoke("get_endpoints");
  settingsState.endpoints = {
    github: backend.github_endpoint || ENDPOINT_DEFAULTS.github,
    gitlab: backend.gitlab_endpoint || ENDPOINT_DEFAULTS.gitlab,
    forgejoHosts: normaliseForgejoHosts(backend.forgejo_trusted_hosts),
  };
}

async function pushSettingsToBackend() {
  await invoke("update_endpoints", {
    githubEndpoint: settingsState.endpoints.github,
    gitlabEndpoint: settingsState.endpoints.gitlab,
    forgejoTrustedHosts: settingsState.endpoints.forgejoHosts,
  });
  await invoke("update_api_keys", {
    githubApiKey: settingsState.keys.github,
    gitlabApiKey: settingsState.keys.gitlab,
    forgejoTokens: settingsState.forgejoTokens,
  });
}

/// Builds the dynamic Forgejo host+token row list inside #settings-forgejo-host-list.
/// Each row has a hostname input, a token password input, and a Remove button.
function renderForgejoHostRows() {
  forgejoHostList.innerHTML = "";
  const hosts = settingsState.endpoints.forgejoHosts;
  const tokens = settingsState.forgejoTokens;
  const showTokens = settingsShowKeys.checked;

  hosts.forEach((entry, idx) => {
    const row = document.createElement("div");
    row.className = "forgejo-host-row";
    row.dataset.idx = idx;

    const hostInput = document.createElement("input");
    hostInput.type = "text";
    hostInput.value = entry.host;
    hostInput.placeholder = "host (e.g. codeberg.org)";
    hostInput.autocomplete = "off";
    hostInput.spellcheck = false;
    hostInput.setAttribute("aria-label", "Forgejo hostname");

    const tokenInput = document.createElement("input");
    tokenInput.type = showTokens ? "text" : "password";
    tokenInput.value = tokens[idx] ?? "";
    tokenInput.placeholder = "token (optional)";
    tokenInput.autocomplete = "off";
    tokenInput.spellcheck = false;
    tokenInput.setAttribute("aria-label", `Token for ${entry.host}`);

    const removeBtn = document.createElement("button");
    removeBtn.type = "button";
    removeBtn.className = "outline";
    removeBtn.textContent = "Remove";
    removeBtn.setAttribute("aria-label", `Remove ${entry.host}`);
    removeBtn.addEventListener("click", () => {
      settingsState.endpoints.forgejoHosts.splice(idx, 1);
      settingsState.forgejoTokens.splice(idx, 1);
      renderForgejoHostRows();
    });

    row.append(hostInput, tokenInput, removeBtn);
    forgejoHostList.appendChild(row);
  });
}

function populateSettingsInputs() {
  for (const field of ["github", "gitlab"]) {
    const input = settingsInputs[field];
    input.value = settingsState.keys[field];
    input.disabled = false;
  }
  settingsInputs.githubEndpoint.value = settingsState.endpoints.github;
  settingsInputs.gitlabEndpoint.value = settingsState.endpoints.gitlab;
  renderForgejoHostRows();
}

settingsShowKeys.addEventListener("change", () => {
  const type = settingsShowKeys.checked ? "text" : "password";
  settingsInputs.github.type = type;
  settingsInputs.gitlab.type = type;
  // Re-render host rows so token inputs pick up the new type.
  renderForgejoHostRows();
});

settingsBtn.addEventListener("click", () => {
  settingsShowKeys.checked = false;
  settingsInputs.github.type = "password";
  settingsInputs.gitlab.type = "password";
  populateSettingsInputs();
  settingsDialog.showModal();
  document.getElementById("settings-sections")?.scrollTo({ top: 0 });
  refreshLogs();
});

forgejoAddHostBtn.addEventListener("click", () => {
  settingsState.endpoints.forgejoHosts.push({ host: "", token_ref: "" });
  settingsState.forgejoTokens.push("");
  renderForgejoHostRows();
  // Focus the new host input.
  const rows = forgejoHostList.querySelectorAll(".forgejo-host-row");
  const lastRow = rows[rows.length - 1];
  lastRow?.querySelector("input")?.focus();
});

// Byte offset of the last byte read from the log file. Persisted across
// refreshLogs() calls within the session for incremental reads (F9).
// Reset to undefined when the user manually triggers a full refresh.
let logNextOffset = undefined;

async function refreshLogs(incremental = false) {
  settingsLogsStatus.textContent = "Loading…";
  try {
    const afterBytes = incremental ? logNextOffset : undefined;
    const chunk = await invoke("get_logs", { limit: 200, afterBytes });
    if (incremental && chunk.lines.length === 0) {
      // No new data — leave the display unchanged.
      settingsLogsStatus.textContent = settingsLogsOutput.textContent
        ? settingsLogsStatus.textContent.replace("Loading…", "").trim() || `${settingsLogsOutput.textContent.split("\n").length} line(s) (no new entries)`
        : "(no log entries yet)";
    } else {
      if (incremental && settingsLogsOutput.textContent && settingsLogsOutput.textContent !== "(no log entries yet)") {
        // Append new lines to the existing display.
        settingsLogsOutput.textContent += "\n" + chunk.lines.join("\n");
      } else {
        settingsLogsOutput.textContent = chunk.lines.length
          ? chunk.lines.join("\n")
          : "(no log entries yet)";
      }
      settingsLogsStatus.textContent = `${settingsLogsOutput.textContent.split("\n").filter(l => l !== "(no log entries yet)").length} line(s)`;
    }
    logNextOffset = chunk.next_offset;
  } catch (e) {
    settingsLogsOutput.textContent = "";
    settingsLogsStatus.textContent = `Error: ${e}`;
    logNextOffset = undefined;
  }
}

settingsLogsRefresh.addEventListener("click", () => {
  // Manual refresh always does a full re-read (resets incremental state).
  logNextOffset = undefined;
  refreshLogs(false);
});


settingsSaveBtn.addEventListener("click", async () => {
  const newKeys = {
    github: settingsInputs.github.value,
    gitlab: settingsInputs.gitlab.value,
  };

  // Read current values from the dynamic Forgejo host+token rows.
  const rows = forgejoHostList.querySelectorAll(".forgejo-host-row");
  const newForgejoHosts = [];
  const newForgejoTokens = [];
  for (const row of rows) {
    const inputs = row.querySelectorAll("input");
    const host = inputs[0].value.trim();
    const token = inputs[1].value;
    if (host) {
      newForgejoHosts.push({
        host,
        token_ref: forgejoVaultKey(host),
      });
      newForgejoTokens.push(token);
    }
  }

  const newEndpoints = {
    github:
      settingsInputs.githubEndpoint.value.trim() || ENDPOINT_DEFAULTS.github,
    gitlab:
      settingsInputs.gitlabEndpoint.value.trim() || ENDPOINT_DEFAULTS.gitlab,
    forgejoHosts: newForgejoHosts,
  };

  settingsSaveBtn.disabled = true;
  settingsSaveBtn.setAttribute("aria-busy", "true");
  try {
    settingsState.keys = newKeys;
    settingsState.endpoints = newEndpoints;
    settingsState.forgejoTokens = newForgejoTokens;
    await pushSettingsToBackend();
    ot.toast("Settings saved", "Done", { variant: "success" });
    settingsDialog.close();
  } catch (e) {
    ot.toast(String(e), "Save failed", { variant: "danger" });
  } finally {
    settingsSaveBtn.disabled = false;
    settingsSaveBtn.removeAttribute("aria-busy");
  }
});

async function initSettings() {
  const endpointErr = await initEndpoints().then(() => null, (e) => e);
  if (endpointErr) {
    console.warn("Endpoint init failed:", endpointErr);
  }
  try {
    await initApiKeys();
  } catch (e) {
    console.warn("Keyring init failed:", e);
    ot.toast(String(e), "API tokens unavailable", { variant: "warning" });
  }
  try {
    settingsAppVersion.textContent = `v${await getVersion()}`;
  } catch {
    settingsAppVersion.textContent = "(unknown)";
  }
}

initSettings();
loadRepos({ showSkeletons: true });
