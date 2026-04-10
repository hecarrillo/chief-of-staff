import { onMount, onCleanup } from "solid-js";
import { mode, setModeSignal, connected } from "../lib/stores";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export default function StatusBar() {
  const isAtDesk = () => mode() === "at_desk";

  onMount(async () => {
    const unlisten = await listen<string>("mode-changed", (event) => {
      setModeSignal(event.payload as "at_desk" | "away");
    });

    onCleanup(() => { unlisten(); });
  });

  async function toggleMode() {
    const newMode = isAtDesk() ? "away" : "at_desk";
    await invoke("set_mode_manual", { mode: newMode });
    setModeSignal(newMode);
  }

  return (
    <div class="flex items-center justify-between px-4 py-2 bg-surface border-t border-neutral-800 text-xs">
      <div class="flex items-center gap-3">
        <span class="flex items-center gap-1.5">
          <span
            class={`w-2 h-2 rounded-full ${connected() ? "bg-terminal-green" : "bg-red-500"}`}
          />
          {connected() ? "Bridge active" : "Disconnected"}
        </span>
        <span class="text-terminal-dim">|</span>
        <span class="text-terminal-dim">localhost:7890</span>
      </div>

      <button
        onClick={toggleMode}
        class={`px-3 py-1 rounded text-xs font-medium transition-colors cursor-pointer ${
          isAtDesk()
            ? "bg-terminal-green/20 text-terminal-green hover:bg-terminal-green/30"
            : "bg-amber-500/20 text-amber-400 hover:bg-amber-500/30"
        }`}
      >
        {isAtDesk() ? "AT DESK" : "AWAY"}
      </button>
    </div>
  );
}
