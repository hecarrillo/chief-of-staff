import { createSignal, onMount, For, onCleanup, Show } from "solid-js";
import { getVaultFiles, readVaultFile } from "../lib/tauri-bridge";
import { onVaultChange } from "../lib/events";
import type { VaultFile } from "../lib/types";
import ProjectCard from "../components/ProjectCard";

const CATEGORIES = [
  { key: "specs", label: "Specs", icon: "S" },
  { key: "decisions", label: "Decisions", icon: "D" },
  { key: "research", label: "Research", icon: "R" },
  { key: "sessions", label: "Sessions", icon: "H" },
] as const;

export default function Dashboard() {
  const [activeTab, setActiveTab] = createSignal<string>("specs");
  const [files, setFiles] = createSignal<VaultFile[]>([]);

  async function loadCategory(cat: string) {
    try {
      setFiles(await getVaultFiles(cat));
    } catch {
      setFiles([]);
    }
  }

  onMount(async () => {
    await loadCategory(activeTab());
    const unlisten = await onVaultChange(async () => {
      await loadCategory(activeTab());
    });
    onCleanup(() => unlisten());
  });

  function switchTab(tab: string) {
    setActiveTab(tab);
    loadCategory(tab);
  }

  return (
    <div class="h-full overflow-y-auto p-4">
      <h2 class="text-sm font-bold text-terminal-green mb-4">Dashboard</h2>

      <div class="flex gap-1 mb-4">
        <For each={CATEGORIES}>
          {(cat) => (
            <button
              onClick={() => switchTab(cat.key)}
              class={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
                activeTab() === cat.key
                  ? "bg-neutral-800 text-neutral-100"
                  : "text-terminal-dim hover:text-neutral-300 hover:bg-surface-hover"
              }`}
            >
              <span class="text-[9px] mr-1 opacity-50">{cat.icon}</span>
              {cat.label}
              <span class="ml-1.5 text-[9px] text-neutral-600">{files().length}</span>
            </button>
          )}
        </For>
      </div>

      <div class="space-y-2">
        <Show
          when={files().length > 0}
          fallback={<p class="text-xs text-neutral-600 italic py-4 text-center">No files in {activeTab()}</p>}
        >
          <For each={files()}>
            {(file) => <ProjectCard file={file} />}
          </For>
        </Show>
      </div>
    </div>
  );
}
