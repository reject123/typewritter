import { invoke } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { listen } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";

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

// 点击文件夹进入
listEl.addEventListener("click", (e) => {
  const li = (e.target as HTMLElement).closest("li");
  if (!li || li.dataset.dir !== "true" || !li.dataset.path) return;
  void render(li.dataset.path);
});

// 可选：Alt + ↑ 返回上级（不需要就删掉这段）
window.addEventListener("keydown", (e) => {
  if (e.altKey && e.key === "ArrowUp") {
    e.preventDefault();
    if (currentParent) void render(currentParent);
  }
});

void render(null);


const term = new Terminal({
  fontFamily: "Consolas, monospace",
  fontSize: 13,
  theme: { background: "#191919", foreground: "#ddd" },
  cursorBlink: true,
  allowTransparency: true,
});

const fit = new FitAddon();
term.loadAddon(fit);
term.open(document.getElementById("terminal")!);

try {
  term.loadAddon(new WebglAddon());
} catch (e) {
  console.warn("WebGL 不可用，回退默认渲染器", e);
}

async function fitTerminal() {
  fit.fit();
  await invoke("resize_pty", { rows: term.rows, cols: term.cols });
}

term.onData((data) => void invoke("write_to_pty", { data }));

async function initTerminal() {
  await listen<string>("pty-output", (ev) => term.write(ev.payload));
  // cwd 传 null 用进程当前目录；想让终端落在文件管理器当前路径，
  // 把下面的 null 换成 currentPath 即可
  await invoke("create_shell", { cwd: null });
  await fitTerminal();
}

window.addEventListener("resize", () => void fitTerminal());
void initTerminal();