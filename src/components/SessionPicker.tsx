import { createSignal, onMount, Show } from "solid-js";
import { currentSession, renewToday, type CurrentSession } from "../lib/tauri-bridge";

/// Compact header showing today's session + Renew button.
/// Past sessions are intentionally NOT listed — we only surface today.
export default function SessionPicker(props: { onChanged?: () => void }) {
  const [info, setInfo] = createSignal<CurrentSession | null>(null);
  const [busy, setBusy] = createSignal(false);

  async function refresh() {
    try {
      const cur = await currentSession();
      setInfo(cur);
    } catch {}
  }

  onMount(refresh);

  async function handleRenew() {
    if (!confirm(`Renew today's session? The current Claude session will be killed and a fresh one started.`)) return;
    setBusy(true);
    try {
      await renewToday();
      await refresh();
      props.onChanged?.();
    } finally {
      setBusy(false);
    }
  }

  const uuid = () => info()?.entry?.uuid.slice(0, 8) || "…";
  const started = () => {
    const c = info()?.entry?.created_at;
    return c ? new Date(c).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "…";
  };

  return (
    <div class="flex items-center gap-2 text-xs">
      <span class="text-[10px] text-neutral-500 uppercase tracking-wider">Session</span>
      <span class="font-mono text-neutral-200">{info()?.today || "…"}</span>
      <span class="text-[9px] text-neutral-600 font-mono">
        {uuid()} · {started()}
      </span>
      <button
        onClick={handleRenew}
        disabled={busy()}
        class="text-[10px] px-2 py-1 border border-neutral-800 rounded text-neutral-400 hover:text-cos-accent hover:border-cos-accent disabled:opacity-50"
        title="Kill today's session and start fresh"
      >
        {busy() ? "…" : "Renew"}
      </button>
    </div>
  );
}
