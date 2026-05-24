import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";
import { appLocalDataDir, join } from "@tauri-apps/api/path";
import { Stronghold } from "@tauri-apps/plugin-stronghold";
import { load as loadStore } from "@tauri-apps/plugin-store";
import "@knadh/oat/oat.min.css";
import "@knadh/oat/oat.min.js";
// State
let editingUrl = null;
let deletingUrl = null;

// Settings runtime state — populated at startup, written back on save.
const VAULT_FILE = "vault.hold";
const VAULT_CLIENT = "api-keys";
const STORE_FILE = "endpoints.store.json";
const VAULT_KEY_NAMES = {
  github: "github_api_key",
  gitlab: "gitlab_api_key",
  forgejo: "forgejo_token",
};
const ENDPOINT_KEY_NAMES = {
  github: "github_endpoint",
  gitlab: "gitlab_endpoint",
  forgejoHosts: "forgejo_trusted_hosts",
};
const ENDPOINT_DEFAULTS = {
  github: "https://api.github.com/repos/",
  gitlab: "https://gitlab.com/api/v4/projects/",
  forgejoHosts: ["codeberg.org"],
};

const settingsState = {
  stronghold: null,
  vaultStore: null,
  endpointStore: null,
  keys: { github: "", gitlab: "", forgejo: "" },
  endpoints: {
    github: ENDPOINT_DEFAULTS.github,
    gitlab: ENDPOINT_DEFAULTS.gitlab,
    forgejoHosts: [...ENDPOINT_DEFAULTS.forgejoHosts],
  },
};

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

function bytesToString(bytes) {
  if (!bytes) return "";
  // Let decode errors propagate — callers must handle them so a corrupted
  // vault entry is distinguishable from an unset one (#45).
  return textDecoder.decode(Uint8Array.from(bytes));
}

function stringToBytes(value) {
  return Array.from(textEncoder.encode(value ?? ""));
}

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
      <dd><code class="js-card-latest" data-copy data-tooltip="Click to copy"></code></dd>
      <dt>Installed version</dt>
      <dd><code class="js-card-system" data-copy data-tooltip="Click to copy"></code></dd>
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
  const host = extractHostUrl(url);

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
    await invoke("add_repo", { url, host, forge });
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

// Extract host URL from a given URL
function extractHostUrl(url) {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

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

  const host_url = extractHostUrl(newUrl);

  try {
    await invoke("edit_repo", { oldUrl, newUrl, host: host_url, forge: newHostKind });
    ot.toast("Repository updated", "Saved", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Update failed", { variant: "danger" });
  }
});

// Mirrors parse_url in git_api_handler.rs — must stay in sync with Rust validation.
// Accepts 2+ path segments so GitLab subgroup URLs work
// (e.g. https://gitlab.com/group/subgroup/project).
function isValidRepoUrl(url) {
  return /^https:\/\/[a-zA-Z0-9._:-]+(\/[a-zA-Z0-9._-]+){2,}\/?$/.test(url);
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
  forgejo: document.getElementById("settings-forgejo-key"),
  githubEndpoint: document.getElementById("settings-github-endpoint"),
  gitlabEndpoint: document.getElementById("settings-gitlab-endpoint"),
  forgejoHosts: document.getElementById("settings-forgejo-hosts"),
};

async function initVault() {
  const vaultKey = await invoke("get_vault_key");
  const vaultPath = await join(await appLocalDataDir(), VAULT_FILE);
  let stronghold;
  try {
    stronghold = await Stronghold.load(vaultPath, vaultKey);
  } catch {
    // vault.hold exists but was encrypted with a different key (e.g. from a
    // build where the OS keyring was not enabled and the key lived only in
    // process memory). Delete the stale file and start fresh — the old data
    // was already inaccessible.
    await invoke("delete_vault_file");
    stronghold = await Stronghold.load(vaultPath, vaultKey);
    ot.toast(
      "Your API keys were stored in an unreadable vault (from an older build) and have been cleared. Please re-enter them in Settings.",
      "Vault reset",
      { variant: "warning" },
    );
  }
  let client;
  try {
    client = await stronghold.loadClient(VAULT_CLIENT);
  } catch {
    client = await stronghold.createClient(VAULT_CLIENT);
  }
  settingsState.stronghold = stronghold;
  settingsState.vaultStore = client.getStore();

  for (const [field, vaultKey] of Object.entries(VAULT_KEY_NAMES)) {
    const stored = await settingsState.vaultStore.get(vaultKey);
    try {
      settingsState.keys[field] = bytesToString(stored);
      settingsState.corruptedKeys = settingsState.corruptedKeys || {};
      settingsState.corruptedKeys[field] = false;
    } catch {
      // Mark this field as corrupted — populateSettingsInputs will disable
      // the input so the user cannot silently overwrite a still-present but
      // unreadable secret (#45).
      settingsState.keys[field] = "";
      settingsState.corruptedKeys = settingsState.corruptedKeys || {};
      settingsState.corruptedKeys[field] = true;
    }
  }
}

async function initEndpoints() {
  const store = await loadStore(STORE_FILE, { autoSave: false });
  settingsState.endpointStore = store;

  const stored = {
    github: await store.get(ENDPOINT_KEY_NAMES.github),
    gitlab: await store.get(ENDPOINT_KEY_NAMES.gitlab),
    forgejoHosts: await store.get(ENDPOINT_KEY_NAMES.forgejoHosts),
  };

  // First-run fallback: read endpoints from config.json (the backend) and
  // seed the Store so subsequent runs read from there directly.
  if (!stored.github && !stored.gitlab && !stored.forgejoHosts) {
    try {
      const backend = await invoke("get_endpoints");
      settingsState.endpoints = {
        github: backend.github_endpoint || ENDPOINT_DEFAULTS.github,
        gitlab: backend.gitlab_endpoint || ENDPOINT_DEFAULTS.gitlab,
        forgejoHosts:
          backend.forgejo_trusted_hosts &&
          backend.forgejo_trusted_hosts.length > 0
            ? backend.forgejo_trusted_hosts
            : [...ENDPOINT_DEFAULTS.forgejoHosts],
      };
      await store.set(
        ENDPOINT_KEY_NAMES.github,
        settingsState.endpoints.github,
      );
      await store.set(
        ENDPOINT_KEY_NAMES.gitlab,
        settingsState.endpoints.gitlab,
      );
      await store.set(
        ENDPOINT_KEY_NAMES.forgejoHosts,
        settingsState.endpoints.forgejoHosts,
      );
      await store.save();
    } catch (e) {
      // Fall through to defaults — backend may not be ready on first launch.
      console.warn("get_endpoints fallback failed:", e);
    }
  } else {
    // Distinguish "null = never saved" from "[] = intentional empty list" (#44).
    // Only fall back to defaults when the key was never written (null/undefined).
    const forgejoHosts =
      stored.forgejoHosts === null || stored.forgejoHosts === undefined
        ? [...ENDPOINT_DEFAULTS.forgejoHosts]
        : Array.isArray(stored.forgejoHosts)
          ? stored.forgejoHosts
          : [...ENDPOINT_DEFAULTS.forgejoHosts];
    settingsState.endpoints = {
      github: stored.github || ENDPOINT_DEFAULTS.github,
      gitlab: stored.gitlab || ENDPOINT_DEFAULTS.gitlab,
      forgejoHosts,
    };
  }
}

async function pushSettingsToBackend() {
  await invoke("update_api_keys", {
    githubApiKey: settingsState.keys.github,
    gitlabApiKey: settingsState.keys.gitlab,
    forgejoToken: settingsState.keys.forgejo,
  });
  await invoke("update_endpoints", {
    githubEndpoint: settingsState.endpoints.github,
    gitlabEndpoint: settingsState.endpoints.gitlab,
    forgejoTrustedHosts: settingsState.endpoints.forgejoHosts,
  });
}

function populateSettingsInputs() {
  const corrupted = settingsState.corruptedKeys || {};
  for (const field of ["github", "gitlab", "forgejo"]) {
    const input = settingsInputs[field];
    if (corrupted[field]) {
      input.value = "";
      input.disabled = true;
      input.placeholder = "⚠ Vault entry corrupted — clear vault to reset";
    } else {
      input.value = settingsState.keys[field];
      input.disabled = false;
    }
  }
  settingsInputs.githubEndpoint.value = settingsState.endpoints.github;
  settingsInputs.gitlabEndpoint.value = settingsState.endpoints.gitlab;
  settingsInputs.forgejoHosts.value =
    settingsState.endpoints.forgejoHosts.join("\n");
}

settingsShowKeys.addEventListener("change", () => {
  const type = settingsShowKeys.checked ? "text" : "password";
  settingsInputs.github.type = type;
  settingsInputs.gitlab.type = type;
  settingsInputs.forgejo.type = type;
});

settingsBtn.addEventListener("click", () => {
  populateSettingsInputs();
  settingsShowKeys.checked = false;
  settingsInputs.github.type = "password";
  settingsInputs.gitlab.type = "password";
  settingsInputs.forgejo.type = "password";
  settingsDialog.showModal();
  document.getElementById("settings-sections")?.scrollTo({ top: 0 });
  refreshLogs();
});

async function refreshLogs() {
  settingsLogsStatus.textContent = "Loading…";
  try {
    const lines = await invoke("get_logs", { limit: 200 });
    settingsLogsOutput.textContent = lines.length
      ? lines.join("\n")
      : "(no log entries yet)";
    settingsLogsStatus.textContent = `${lines.length} line(s)`;
  } catch (e) {
    settingsLogsOutput.textContent = "";
    settingsLogsStatus.textContent = `Error: ${e}`;
  }
}

settingsLogsRefresh.addEventListener("click", refreshLogs);

function parseHosts(text) {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

settingsSaveBtn.addEventListener("click", async () => {
  const newKeys = {
    github: settingsInputs.github.value,
    gitlab: settingsInputs.gitlab.value,
    forgejo: settingsInputs.forgejo.value,
  };
  const newEndpoints = {
    github:
      settingsInputs.githubEndpoint.value.trim() || ENDPOINT_DEFAULTS.github,
    gitlab:
      settingsInputs.gitlabEndpoint.value.trim() || ENDPOINT_DEFAULTS.gitlab,
    forgejoHosts: parseHosts(settingsInputs.forgejoHosts.value),
  };

  settingsSaveBtn.disabled = true;
  settingsSaveBtn.setAttribute("aria-busy", "true");
  try {
    // Persist secrets to Stronghold — vault must be initialised.
    if (!settingsState.vaultStore || !settingsState.stronghold) {
      throw new Error(
        "Vault unavailable — API tokens not saved. Check the warning shown at startup.",
      );
    }
    for (const [field, vaultKey] of Object.entries(VAULT_KEY_NAMES)) {
      await settingsState.vaultStore.insert(
        vaultKey,
        stringToBytes(newKeys[field]),
      );
    }
    await settingsState.stronghold.save();

    // Persist endpoints to Store — endpoint store must also be ready.
    if (!settingsState.endpointStore) {
      throw new Error("Endpoint store unavailable — endpoints not saved.");
    }
    await settingsState.endpointStore.set(
      ENDPOINT_KEY_NAMES.github,
      newEndpoints.github,
    );
    await settingsState.endpointStore.set(
      ENDPOINT_KEY_NAMES.gitlab,
      newEndpoints.gitlab,
    );
    await settingsState.endpointStore.set(
      ENDPOINT_KEY_NAMES.forgejoHosts,
      newEndpoints.forgejoHosts,
    );
    await settingsState.endpointStore.save();

    settingsState.keys = newKeys;
    settingsState.endpoints = newEndpoints;
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
  try {
    await initVault();
  } catch (e) {
    console.warn("Stronghold init failed:", e);
    ot.toast(String(e), "Vault unavailable — API tokens will not persist", {
      variant: "warning",
    });
  }
  try {
    await initEndpoints();
  } catch (e) {
    console.warn("Store init failed:", e);
  }
  // Push only API keys to the backend at startup (tokens never persist to
  // config.json — they must be loaded from the vault each launch). Endpoints
  // are intentionally NOT pushed here: update_endpoints is write-on-change
  // in Rust, but the config.json read on startup already loads them, so an
  // unconditional push here was clobbering hand-edited config.json on every
  // launch (#38). Endpoint updates travel to the backend only when the user
  // explicitly saves settings.
  try {
    await invoke("update_api_keys", {
      githubApiKey: settingsState.keys.github,
      gitlabApiKey: settingsState.keys.gitlab,
      forgejoToken: settingsState.keys.forgejo,
    });
  } catch (e) {
    console.warn("Pushing API keys to backend failed:", e);
  }
  try {
    settingsAppVersion.textContent = `v${await getVersion()}`;
  } catch {
    settingsAppVersion.textContent = "(unknown)";
  }
}

initSettings();
loadRepos({ showSkeletons: true });
