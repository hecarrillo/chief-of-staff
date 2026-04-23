import { createSignal, onMount, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

interface LaunchState {
  today: string;
  entry: {
    date: string;
    uuid: string;
    created_at: string;
  } | null;
  needs_prompt: boolean;
  framework_stale: boolean;
  launched: boolean;
}

/// Blocks the UI until the user resolves the launch prompt.
/// Shown every time the app opens — required to create/resume today's tmux session.
export default function SessionLaunchPrompt(props: { onReady: () => void }) {
  const [state, setState] = createSignal<LaunchState | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [choice, setChoice] = createSignal<"continue" | "renew" | null>(null);
  const [dismissed, setDismissed] = createSignal(false);

  onMount(async () => {
    try {
      const s = await invoke<LaunchState>("get_launch_state");
      setState(s);
      if (s.launched) {
        // Already launched in this process — skip prompt.
        setDismissed(true);
        props.onReady();
        return;
      }
      if (!s.needs_prompt) {
        // No prior entry → auto-launch fresh, no prompt needed.
        await launch("renew");
      }
    } catch (err) {
      console.error("Failed to get launch state:", err);
    }
  });

  async function launch(mode: "continue" | "renew") {
    setBusy(true);
    setChoice(mode);
    try {
      await invoke("launch_session", { mode });
      setDismissed(true);
      props.onReady();
    } catch (err) {
      console.error("launch_session failed:", err);
      alert(`Failed to launch session: ${err}`);
    } finally {
      setBusy(false);
      setChoice(null);
    }
  }

  return (
    <Show when={!dismissed() && state()?.needs_prompt && !state()?.launched}>
      <div class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-6">
        <div class="bg-neutral-900 border border-neutral-700 rounded-lg max-w-md w-full p-5 shadow-2xl">
          <h3 class="text-sm font-bold text-terminal-green mb-1">Welcome back</h3>
          <p class="text-xs text-neutral-400 mb-1">
            Today's session: <span class="font-mono text-neutral-200">{state()!.today}</span>
          </p>
          <Show when={state()!.entry}>
            <p class="text-[10px] text-neutral-500 mb-4">
              Started{" "}
              <span class="font-mono">
                {new Date(state()!.entry!.created_at).toLocaleTimeString()}
              </span>
              {" · "}
              uuid <span class="font-mono">{state()!.entry!.uuid.slice(0, 8)}</span>
            </p>
          </Show>

          <Show when={state()!.framework_stale}>
            <div class="mb-4 p-2.5 bg-amber-950/40 border border-amber-700/50 rounded text-[11px] text-amber-200">
              The CoS framework has been updated since this session was created.
              Continuing keeps the old framework — Renew to load the latest.
            </div>
          </Show>

          <div class="flex flex-col gap-2">
            <button
              onClick={() => launch("continue")}
              disabled={busy()}
              class="w-full px-3 py-2 bg-neutral-800 text-neutral-200 rounded text-xs hover:bg-neutral-700 disabled:opacity-50 transition-colors"
            >
              {busy() && choice() === "continue" ? "Resuming…" : "Continue previous session"}
            </button>
            <button
              onClick={() => launch("renew")}
              disabled={busy()}
              class="w-full px-3 py-2 bg-cos-accent/20 text-cos-accent border border-cos-accent/40 rounded text-xs hover:bg-cos-accent/30 disabled:opacity-50 transition-colors"
            >
              {busy() && choice() === "renew" ? "Creating fresh…" : "Start fresh (Renew)"}
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
}
