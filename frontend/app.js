import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import "@knadh/oat/oat.min.css";
import "@knadh/oat/oat.min.js";
// State
let editingUrl = null;

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
  if (editDialog.returnValue !== "save") {
    editingUrl = null;
    return;
  }

  const newUrl = editUrlInput.value.trim();
  const newHostKind = editHostSelect.value;

  if (!editingUrl || !newUrl) {
    editingUrl = null;
    return;
  }
  if (!isValidRepoUrl(newUrl)) {
    ot.toast("URL must be https://host/owner/repo", "Invalid URL", {
      variant: "danger",
    });
    editingUrl = null;
    return;
  }

  const host_url = extractHostUrl(newUrl);

  try {
    await invoke("delete_repo", { url: editingUrl });
    await invoke("add_repo", { url: newUrl, host: host_url, forge: newHostKind });
    ot.toast("Repository updated", "Saved", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Update failed", { variant: "danger" });
  } finally {
    editingUrl = null;
  }
});

// Mirrors parse_url in git_api_handler.rs — must stay in sync with Rust validation.
function isValidRepoUrl(url) {
  return /^https:\/\/[a-zA-Z0-9._:-]+\/[a-zA-Z0-9._-]+\/[a-zA-Z0-9._-]+\/?$/.test(
    url,
  );
}

loadRepos();
