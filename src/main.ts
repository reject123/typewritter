import { invoke } from "@tauri-apps/api/core";

interface EntryInfo {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

interface Listing {
  current: string;
  parent: string | null;
  entries: EntryInfo[];
}

const pathEl = document.getElementById("path")!;
const listEl = document.getElementById("list")!;
const upBtn = document.getElementById("up") as HTMLButtonElement;

let currentParent: string | null = null;

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[c]!));
}

async function render(path?: string | null) {
  try {
    const listing = await invoke<Listing>("list_dir", { path: path ?? null });
    currentParent = listing.parent;
    pathEl.textContent = listing.current;
    upBtn.disabled = !listing.parent;

    listEl.innerHTML = "";
    for (const entry of listing.entries) {
      const li = document.createElement("li");
      li.dataset.path = entry.path;
      li.dataset.dir = String(entry.is_dir);
      li.innerHTML = `
        <span class="icon">${entry.is_dir ? "📁" : "📄"}</span>
        <span class="name">${escapeHtml(entry.name)}</span>
        <span class="size">${entry.is_dir ? "" : formatSize(entry.size)}</span>
      `;
      listEl.appendChild(li);
    }
  } catch (e) {
    alert(`读取目录失败：${e}`);
  }
}

listEl.addEventListener("click", (e) => {
  const li = (e.target as HTMLElement).closest("li");
  if (!li || li.dataset.dir !== "true" || !li.dataset.path) return;
  void render(li.dataset.path);
});

upBtn.addEventListener("click", () => {
  if (currentParent) void render(currentParent);
});

// 初始打开当前工作目录（Tauri dev 下通常是项目根目录）
void render(null);