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
  try {
    return textDecoder.decode(Uint8Array.from(bytes));
  } catch {
    return "";
  }
}

function stringToBytes(value) {
  return Array.from(textEncoder.encode(value ?? ""));
}

// DOM refs
const repoGrid = document.getElementById("repo-grid");
const urlInput = document.getElementById("repo-url-input");
const addBtn = document.getElementById("add-repo-btn");
const refreshAllBtn = document.getElementById("refresh-all-btn");
const editDialog = document.getElementById("edit-dialog");
const editUrlInput = document.getElementById("edit-url-input");
const hostSelect = document.getElementById("repo-host-select");
const editHostSelect = document.getElementById("edit-host-select");
const editCancelBtn = document.getElementById("edit-cancel-btn");

editCancelBtn.addEventListener("click", () => editDialog.close());

// Delegated handler for all card action buttons — one listener for all cards.
repoGrid.addEventListener("click", (e) => {
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
    case "delete-trigger":
      card.querySelector(".delete-popover").hidden = false;
      break;
    case "delete-cancel":
      card.querySelector(".delete-popover").hidden = true;
      break;
    case "confirm-delete":
      card.querySelector(".delete-popover").hidden = true;
      handleDelete(url);
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
  const badgeClass = isUpToDate ? "success" : "warning";
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
  const hostSlug =
    data.host_kind?.toLowerCase().replace("compatible", "") || "unknown";

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
        <span class="js-card-host-badge host-badge"></span>
        <a class="repo-url js-card-url"></a>
      </div>
    </header>
    <div class="repo-card-meta">
      <span>Latest release</span>
      <code class="js-card-latest"></code>
    </div>
    <div class="repo-card-meta">
      <span>Installed version</span>
      <code class="js-card-system"></code>
    </div>
    <footer class="repo-card-actions">
      <button class="small outline" data-action="refresh">Refresh</button>
      <button class="small outline" data-action="mark-updated">Mark as updated</button>
      <button class="small outline" data-action="edit">Edit</button>
      <button class="small outline" data-variant="danger" data-action="delete-trigger">Delete</button>
      <article class="card delete-popover" hidden>
        <header>
          <h4>Delete repo?</h4>
          <p>This cannot be undone.</p>
        </header>
        <br />
        <footer>
          <button class="outline small" data-action="delete-cancel">Cancel</button>
          <button data-variant="danger" class="small" data-action="confirm-delete">Confirm delete</button>
        </footer>
      </article>
    </footer>
  `;

  // Inject all dynamic data via DOM — never via innerHTML interpolation
  card.querySelector(".js-card-name").textContent = name;
  const badge = card.querySelector(".js-card-badge");
  badge.className = `badge ${badgeClass}`;
  badge.textContent = badgeText;
  const hostBadge = card.querySelector(".js-card-host-badge");
  hostBadge.classList.add(`host-badge--${hostSlug}`);
  hostBadge.textContent = forgeLabel(data.host_kind);
  const link = card.querySelector(".js-card-url");
  if (safeUrl) link.href = safeUrl;
  link.textContent = shortUrl;
  card.querySelector(".js-card-latest").textContent = latest;
  card.querySelector(".js-card-system").textContent = system;
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

// Load all repos
async function loadRepos() {
  try {
    const repos = await invoke("get_repos");
    repoGrid.innerHTML = "";
    Object.entries(repos).forEach(([url, data]) => {
      repoGrid.appendChild(buildCard(url, data));
    });
  } catch (e) {
    ot.toast(String(e), "Failed to load repos", { variant: "danger" });
  }
}

// Add repo
addBtn.addEventListener("click", async () => {
  const url = urlInput.value.trim();
  const forge = hostSelect.value;
  const host = extractHostUrl(url);

  if (!url) {
    ot.toast("Please enter a repository URL", "Missing URL", {
      variant: "warning",
    });
    return;
  }
  if (!isValidRepoUrl(url)) {
    ot.toast("URL must be https://host/owner/repo", "Invalid URL", {
      variant: "danger",
    });
    return;
  }

  addBtn.disabled = true;
  try {
    await invoke("add_repo", { url, host, forge });
    ot.toast("Repository added", "Done", { variant: "success" });
    urlInput.value = "";
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Add repo failed", { variant: "danger" });
  } finally {
    addBtn.disabled = false;
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
  try {
    await invoke("refresh_repo", { url });
    ot.toast("Repo refreshed", "Done", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Refresh failed", { variant: "danger" });
  } finally {
    btn.disabled = false;
  }
}

// Refresh all repos
refreshAllBtn.addEventListener("click", async () => {
  refreshAllBtn.disabled = true;
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
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Refresh all failed", { variant: "danger" });
  } finally {
    refreshAllBtn.disabled = false;
  }
});

// Delete repo
async function handleDelete(url) {
  try {
    await invoke("delete_repo", { url });
    document.querySelector(`[data-url="${url}"]`)?.remove();
    ot.toast("Repository deleted", "", { variant: "success" });
  } catch (e) {
    ot.toast(String(e), "Delete failed", { variant: "danger" });
  }
}

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
const settingsTabs = document.getElementById("settings-tabs");
const settingsSections = document.querySelectorAll(".settings-section");
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
  const stronghold = await Stronghold.load(vaultPath, vaultKey);
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
    settingsState.keys[field] = bytesToString(stored);
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
    settingsState.endpoints = {
      github: stored.github || ENDPOINT_DEFAULTS.github,
      gitlab: stored.gitlab || ENDPOINT_DEFAULTS.gitlab,
      forgejoHosts:
        Array.isArray(stored.forgejoHosts) && stored.forgejoHosts.length > 0
          ? stored.forgejoHosts
          : [...ENDPOINT_DEFAULTS.forgejoHosts],
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
  settingsInputs.github.value = settingsState.keys.github;
  settingsInputs.gitlab.value = settingsState.keys.gitlab;
  settingsInputs.forgejo.value = settingsState.keys.forgejo;
  settingsInputs.githubEndpoint.value = settingsState.endpoints.github;
  settingsInputs.gitlabEndpoint.value = settingsState.endpoints.gitlab;
  settingsInputs.forgejoHosts.value =
    settingsState.endpoints.forgejoHosts.join("\n");
}

function activateTab(name) {
  settingsTabs.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.tab === name);
  });
  settingsSections.forEach((section) => {
    section.hidden = section.dataset.section !== name;
  });
}

settingsTabs.addEventListener("click", (e) => {
  const btn = e.target.closest(".tab-btn");
  if (!btn) return;
  activateTab(btn.dataset.tab);
  if (btn.dataset.tab === "logs") {
    refreshLogs();
  }
});

settingsShowKeys.addEventListener("change", () => {
  const type = settingsShowKeys.checked ? "text" : "password";
  settingsInputs.github.type = type;
  settingsInputs.gitlab.type = type;
  settingsInputs.forgejo.type = type;
});

settingsBtn.addEventListener("click", () => {
  populateSettingsInputs();
  activateTab("api-keys");
  settingsShowKeys.checked = false;
  settingsInputs.github.type = "password";
  settingsInputs.gitlab.type = "password";
  settingsInputs.forgejo.type = "password";
  settingsDialog.showModal();
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
loadRepos();
