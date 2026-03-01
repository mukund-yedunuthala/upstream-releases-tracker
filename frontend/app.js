import { invoke } from "@tauri-apps/api/core";

// State
let editingUrl = null;
let editingHostKind = null;

// DOM refs
const repoGrid = document.getElementById("repo-grid");
const urlInput = document.getElementById("repo-url-input");
const addBtn = document.getElementById("add-repo-btn");
const refreshAllBtn = document.getElementById("refresh-all-btn");
const editDialog = document.getElementById("edit-dialog");
const editUrlInput = document.getElementById("edit-url-input");
const hostSelect = document.getElementById("repo-host-select");
const editHostSelect = document.getElementById("edit-host-select");

// Maps ForgeKind enum variants to display labels
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

// Build a card from RepoData
function buildCard(url, data) {
  const id = btoa(url)
    .replace(/[^a-zA-Z0-9]/g, "")
    .slice(0, 12);
  const isUpToDate =
    !!data.system_version && data.system_version === data.latest_release;
  const badgeClass = isUpToDate ? "success" : "warning";
  const badgeText = isUpToDate ? "Up to date" : "Update available";
  const shortUrl = url.replace(/^https?:\/\//, "");
  const title = data.owner + "/" + data.repo_name;
  const name = title || shortUrl;
  const latest = data.latest_release || "—";
  const system = data.system_version || "—";

  const card = document.createElement("article");
  card.className = "card repo-card";
  card.dataset.url = url;

  card.innerHTML = `
    <header>
      <div class="repo-card-title">
        <strong>${name}</strong>
        <span class="badge ${badgeClass}">${badgeText}</span>
      </div>
      <div class="repo-card-subtitle">
        <span class="host-badge host-badge--${data.host_kind?.toLowerCase().replace("compatible", "") || "unknown"}">
          ${forgeLabel(data.host_kind)}
        </span>
        <a href="${url}" class="repo-url">${shortUrl}</a>
      </div>
    </header>
    <div class="repo-card-meta">
      <span>Latest release</span>
      <code>${latest}</code>
    </div>
    <div class="repo-card-meta">
      <span>Installed version</span>
      <code>${system}</code>
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

  // Refresh
  card
    .querySelector('[data-action="refresh"]')
    .addEventListener("click", (e) => handleRefresh(e.currentTarget, url));

  // Mark as updated
  card
    .querySelector('[data-action="mark-updated"]')
    .addEventListener("click", () => handleMarkAsUpdated(url));

  // Edit
  card
    .querySelector('[data-action="edit"]')
    .addEventListener("click", () => openEditDialog(url, name, data.host));

  // Delete trigger: show the inline popover
  const deletePopover = card.querySelector(".delete-popover");
  card
    .querySelector('[data-action="delete-trigger"]')
    .addEventListener("click", () => {
      deletePopover.hidden = false;
    });

  // Cancel: hide it again
  card
    .querySelector('[data-action="delete-cancel"]')
    .addEventListener("click", () => {
      deletePopover.hidden = true;
    });

  // Confirm delete
  card
    .querySelector('[data-action="confirm-delete"]')
    .addEventListener("click", () => {
      deletePopover.hidden = true;
      handleDelete(url);
    });

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
  const forge = hostSelect.value; // "GitHub" | "GitLab" | "ForgejoCompatible"
  const host = extractHostUrl(url); // "github.com", "codeberg.org", etc.

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
    const repos = await invoke("get_repos");
    const urls = Object.keys(repos);
    for (const url of urls) {
      await invoke("refresh_repo", { url });
    }
    ot.toast("All repos refreshed", "Done", { variant: "success" });
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
function openEditDialog(url, name, hostKind) {
  editingUrl = url;
  editingHostKind = hostKind;
  editUrlInput.value = url;
  editHostSelect.value = hostKind;
  editDialog.showModal();
}
// Edit dialog
editDialog.addEventListener("close", async () => {
  if (editDialog.returnValue !== "save") {
    editingUrl = null;
    editingHostKind = null;
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
    await invoke("add_repo", { url: newUrl, host_url, host_kind: newHostKind });
    ot.toast("Repository updated", "Saved", { variant: "success" });
    await loadRepos();
  } catch (e) {
    ot.toast(String(e), "Update failed", { variant: "danger" });
  } finally {
    editingUrl = null;
    editingHostKind = null;
  }
});

// Mirrors helper.rs is_valid_repo_url — any HTTPS URL with owner/repo segments.
function isValidRepoUrl(url) {
  return /^https:\/\/[a-zA-Z0-9._:-]+\/[a-zA-Z0-9._-]+\/[a-zA-Z0-9._-]+\/?$/.test(
    url,
  );
}

// Initial load
loadRepos();
