import { createSignal, onMount, onCleanup, For, Show } from "solid-js";
import { getSessions, getWindows, getScreen, sendToSession } from "../lib/tauri-bridge";
import type { SessionInfo, WindowInfo } from "../lib/types";

export default function Sessions() {
  const [sessions, setSessions] = createSignal<SessionInfo[]>([]);
  const [activeTarget, setActiveTarget] = createSignal<string | null>(null);
  const [activeWindows, setActiveWindows] = createSignal<WindowInfo[]>([]);
  const [screenContent, setScreenContent] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [autoRefresh, setAutoRefresh] = createSignal(true);
  let inputRef!: HTMLInputElement;
  let screenRef: HTMLPreElement | undefined;
  let refreshTimer: ReturnType<typeof setInterval> | undefined;

  onMount(async () => {
    await refresh();
    // Auto-refresh screen every 2s
    refreshTimer = setInterval(() => {
      if (autoRefresh() && activeTarget()) {
        refreshScreen();
      }
    }, 2000);
  });

  onCleanup(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });

  async function refresh() {
    setLoading(true);
    try {
      const list = await getSessions();
      setSessions(list);
      // Auto-select first session if none active
      if (!activeTarget() && list.length > 0) {
        await selectSession(list[0].name);
      }
    } catch {
      setSessions([]);
    } finally {
      setLoading(false);
    }
  }

  async function selectSession(name: string) {
    try {
      const wins = await getWindows(name);
      setActiveWindows(wins);
      const target = wins.length <= 1 ? name : `${name}:${wins[0].name}`;
      setActiveTarget(target);
      await refreshScreen(target);
    } catch {
      setActiveWindows([]);
      setActiveTarget(name);
    }
  }

  async function selectWindow(session: string, win: WindowInfo) {
    const target = `${session}:${win.name}`;
    setActiveTarget(target);
    await refreshScreen(target);
  }

  async function refreshScreen(target?: string) {
    const t = target || activeTarget();
    if (!t) return;
    try {
      const content = await getScreen(t, 60);
      setScreenContent(content);
      // Auto-scroll to bottom
      requestAnimationFrame(() => {
        if (screenRef) screenRef.scrollTop = screenRef.scrollHeight;
      });
    } catch (err) {
      setScreenContent(`(error: ${err})`);
    }
  }

  async function handleSend() {
    const target = activeTarget();
    const text = inputRef.value.trim();
    if (!target || !text) return;
    try {
      await sendToSession(target, text);
      inputRef.value = "";
      // Refresh screen after a short delay to see the result
      setTimeout(() => refreshScreen(), 500);
    } catch (err) {
      console.error("Send failed:", err);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSend();
    }
  }

  const activeSession = () => activeTarget()?.split(":")[0] || "";

  return (
    <div class="flex h-full">
      {/* Left panel: session list */}
      <div class="w-56 border-r border-neutral-800 flex flex-col bg-surface">
        <div class="px-3 py-3 border-b border-neutral-800 flex items-center justify-between">
          <h3 class="text-xs font-bold text-terminal-green">Sessions</h3>
          <button
            onClick={refresh}
            class="text-[10px] text-neutral-500 hover:text-neutral-300"
          >
            {loading() ? "..." : "↻"}
          </button>
        </div>

        <div class="flex-1 overflow-y-auto">
          <Show
            when={sessions().length > 0}
            fallback={
              <p class="text-[10px] text-neutral-600 p-3 text-center">
                {loading() ? "Loading..." : "No sessions"}
              </p>
            }
          >
            <For each={sessions()}>
              {(sess) => (
                <div>
                  <button
                    onClick={() => selectSession(sess.name)}
                    class={`w-full flex items-center gap-2 px-3 py-2 text-xs transition-colors ${
                      activeSession() === sess.name
                        ? "bg-neutral-800 text-neutral-100"
                        : "text-neutral-400 hover:bg-surface-hover hover:text-neutral-200"
                    }`}
                  >
                    <span class="w-2 h-2 rounded-full bg-terminal-green/50 shrink-0" />
                    <span class="truncate flex-1 text-left">{sess.name}</span>
                    <span class="text-[9px] text-neutral-600">{sess.windows}w</span>
                  </button>
                  {/* Show windows when session is selected */}
                  <Show when={activeSession() === sess.name && activeWindows().length > 1}>
                    <For each={activeWindows()}>
                      {(win) => (
                        <button
                          onClick={() => selectWindow(sess.name, win)}
                          class={`w-full flex items-center gap-2 pl-7 pr-3 py-1 text-[11px] transition-colors ${
                            activeTarget() === `${sess.name}:${win.name}`
                              ? "text-terminal-green bg-neutral-800/50"
                              : "text-neutral-500 hover:text-neutral-300"
                          }`}
                        >
                          <span class="text-[9px]">{win.index}</span>
                          <span class="truncate">{win.name}</span>
                          {win.active && <span class="text-[8px] text-terminal-green ml-auto">*</span>}
                        </button>
                      )}
                    </For>
                  </Show>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>

      {/* Right panel: terminal view */}
      <div class="flex-1 flex flex-col">
        <Show
          when={activeTarget()}
          fallback={
            <div class="flex-1 flex items-center justify-center text-neutral-600 text-xs">
              Select a session to view
            </div>
          }
        >
          {/* Header */}
          <div class="flex items-center gap-2 px-4 py-2 border-b border-neutral-800 bg-neutral-950/50">
            <span class="text-xs text-terminal-green font-medium">{activeTarget()}</span>
            <div class="ml-auto flex items-center gap-2">
              <label class="flex items-center gap-1.5 text-[10px] text-neutral-500 cursor-pointer">
                <input
                  type="checkbox"
                  checked={autoRefresh()}
                  onChange={(e) => setAutoRefresh(e.currentTarget.checked)}
                  class="w-3 h-3 accent-terminal-green"
                />
                Auto-refresh
              </label>
              <button
                onClick={() => refreshScreen()}
                class="text-[10px] text-neutral-500 hover:text-neutral-300 px-1"
              >
                ↻
              </button>
            </div>
          </div>

          {/* Terminal output */}
          <pre
            ref={screenRef}
            class="flex-1 overflow-auto px-4 py-3 text-[11px] text-neutral-300 bg-neutral-950 font-mono leading-relaxed whitespace-pre-wrap"
          >
            {screenContent() || "(empty)"}
          </pre>

          {/* Input */}
          <div class="border-t border-neutral-800 px-3 py-2 bg-surface">
            <div class="flex gap-2 items-center">
              <span class="text-terminal-green text-xs">$</span>
              <input
                ref={inputRef}
                onKeyDown={handleKeyDown}
                placeholder="Type and press Enter to send to session..."
                class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-100 placeholder-neutral-600 font-mono focus:outline-none focus:border-terminal-green"
              />
              <button
                onClick={handleSend}
                class="px-3 py-1.5 bg-terminal-green/20 text-terminal-green rounded text-xs font-medium hover:bg-terminal-green/30 transition-colors"
              >
                Send
              </button>
            </div>
            <p class="text-[9px] text-neutral-600 mt-1">
              Enter sends text + Enter key to the tmux session
            </p>
          </div>
        </Show>
      </div>
    </div>
  );
}
