import { createSignal, onMount, onCleanup, For, Show } from "solid-js";
import { marked } from "marked";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { readDailyNote, writeDailyNote, parseDaily, toggleTodoAt } from "../lib/daily";

marked.setOptions({ gfm: true, breaks: false });

const MIN_WIDTH = 260;
const MAX_WIDTH = 720;
const DEFAULT_WIDTH = 340;

export default function TodayPanel() {
  const [content, setContent] = createSignal("");
  const [path, setPath] = createSignal("");
  const [date, setDate] = createSignal("");
  const [mode, setMode] = createSignal<"view" | "edit">("view");
  const [draft, setDraft] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);
  const stored = parseInt(localStorage.getItem("today-panel-width") || "") || DEFAULT_WIDTH;
  const [width, setWidth] = createSignal(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, stored)));
  let editorRef: HTMLTextAreaElement | undefined;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  async function load() {
    try {
      const note = await readDailyNote();
      setPath(note.path);
      setDate(note.date);
      setContent(note.content);
      if (mode() === "edit" && !dirty()) setDraft(note.content);
    } catch (err) {
      console.error("readDailyNote failed:", err);
    }
  }

  async function persist(newContent: string) {
    setSaving(true);
    try {
      await writeDailyNote(newContent);
      setContent(newContent);
      setDirty(false);
    } catch (err) {
      console.error("writeDailyNote failed:", err);
    } finally {
      setSaving(false);
    }
  }

  async function toggleAt(lineIdx: number) {
    const next = toggleTodoAt(content(), lineIdx);
    if (next === content()) return;
    await persist(next);
  }

  function enterEditMode() {
    setDraft(content());
    setDirty(false);
    setMode("edit");
    requestAnimationFrame(() => editorRef?.focus());
  }

  async function saveDraft() {
    if (!dirty()) { setMode("view"); return; }
    await persist(draft());
    setMode("view");
  }

  function handleEditInput(e: Event) {
    const val = (e.currentTarget as HTMLTextAreaElement).value;
    setDraft(val);
    setDirty(true);
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      if (dirty()) persist(val);
    }, 1200);
  }

  function handleEditorKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      saveDraft();
    }
    if (e.key === "Escape") {
      setMode("view");
    }
  }

  async function openInObsidian() {
    try {
      const url = `obsidian://open?path=${encodeURIComponent(path())}`;
      await invoke("plugin:opener|open_url", { url });
    } catch {
      // Fallback: just open the file
      try { await invoke("plugin:opener|open_path", { path: path() }); } catch {}
    }
  }

  // Resize handle
  function startResize(e: MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = width();
    const onMove = (ev: MouseEvent) => {
      const dx = startX - ev.clientX;
      const next = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startW + dx));
      setWidth(next);
    };
    const onUp = () => {
      localStorage.setItem("today-panel-width", String(width()));
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  onMount(async () => {
    await load();
    const unlisten = await listen<{ path: string }>("vault-change", (event) => {
      // Reload if today's daily was externally modified (Obsidian, cli, etc.)
      if (event.payload.path === path()) {
        if (mode() === "edit" && dirty()) return; // don't clobber user's draft
        load();
      }
    });
    onCleanup(() => {
      unlisten();
      clearTimeout(saveTimer);
    });
  });

  const parsed = () => parseDaily(content());

  return (
    <div class="flex h-full relative" style={{ width: `${width()}px` }}>
      {/* Resize handle on the LEFT edge */}
      <div
        onMouseDown={startResize}
        class="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-cos-accent/40 z-10"
        title="Drag to resize"
      />

      <div class="flex-1 flex flex-col border-l border-neutral-800 bg-surface overflow-hidden">
        {/* Header */}
        <div class="flex items-center gap-2 px-3 py-2 border-b border-neutral-800">
          <span class="text-[10px] text-terminal-green font-bold uppercase tracking-wider">Today</span>
          <span class="text-[10px] text-neutral-500 font-mono">{date() || "…"}</span>
          <div class="ml-auto flex items-center gap-1">
            <Show when={saving()}>
              <span class="text-[9px] text-cos-accent animate-pulse">saving…</span>
            </Show>
            <Show when={mode() === "view"}>
              <button
                onClick={enterEditMode}
                class="text-[10px] px-2 py-0.5 rounded text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800"
                title="Edit raw markdown"
              >Edit</button>
            </Show>
            <Show when={mode() === "edit"}>
              <button
                onClick={saveDraft}
                class="text-[10px] px-2 py-0.5 rounded text-cos-accent hover:bg-cos-accent/20"
                title="Save (Cmd+S)"
              >Save</button>
            </Show>
            <button
              onClick={load}
              class="text-[10px] px-1 text-neutral-500 hover:text-neutral-300"
              title="Reload from disk"
            >↻</button>
            <button
              onClick={openInObsidian}
              class="text-[10px] px-1 text-neutral-500 hover:text-neutral-300"
              title="Open in Obsidian"
            >⎋</button>
          </div>
        </div>

        {/* Body */}
        <div class="flex-1 overflow-auto">
          <Show when={mode() === "edit"}>
            <textarea
              ref={editorRef}
              value={draft()}
              onInput={handleEditInput}
              onKeyDown={handleEditorKey}
              spellcheck={false}
              class="w-full h-full bg-neutral-950 text-neutral-100 text-xs font-mono p-3 resize-none focus:outline-none leading-relaxed"
            />
          </Show>

          <Show when={mode() === "view"}>
            <div class="px-3 py-3 space-y-4 text-xs">
              {/* Todos */}
              <section>
                <h4 class="text-[10px] uppercase tracking-wider text-terminal-green font-bold mb-1.5">
                  Todos{parsed().todos.length > 0 && <span class="text-neutral-600 ml-1 font-normal">({parsed().todos.filter(t => !t.checked).length}/{parsed().todos.length})</span>}
                </h4>
                <Show when={parsed().todos.length > 0} fallback={<p class="text-[10px] text-neutral-600 italic">No todos yet</p>}>
                  <ul class="space-y-1">
                    <For each={parsed().todos}>
                      {(t) => (
                        <li class="flex items-start gap-2 group">
                          <input
                            type="checkbox"
                            checked={t.checked}
                            onChange={() => toggleAt(t.line)}
                            class="mt-0.5 accent-terminal-green cursor-pointer"
                          />
                          <span classList={{ "line-through text-neutral-600": t.checked, "text-neutral-200": !t.checked }}>
                            {t.text}
                          </span>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              {/* Log */}
              <section>
                <h4 class="text-[10px] uppercase tracking-wider text-terminal-green font-bold mb-1.5">Log</h4>
                <Show when={parsed().log.length > 0} fallback={<p class="text-[10px] text-neutral-600 italic">No entries yet</p>}>
                  <ul class="space-y-2">
                    <For each={parsed().log}>
                      {(entry) => (
                        <li class="text-neutral-300">
                          <span class="text-[10px] text-terminal-green font-mono mr-1.5">{entry.time}</span>
                          <span class="text-[11px]">{entry.body.replace(/#[\w:-]+/g, "").trim()}</span>
                          <Show when={entry.tags.length > 0}>
                            <div class="flex flex-wrap gap-1 mt-1">
                              <For each={entry.tags}>
                                {(tag) => (
                                  <span class="text-[9px] px-1.5 py-0.5 rounded bg-cos-accent/10 text-cos-accent border border-cos-accent/20 font-mono">
                                    {tag}
                                  </span>
                                )}
                              </For>
                            </div>
                          </Show>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>

              {/* Open */}
              <section>
                <h4 class="text-[10px] uppercase tracking-wider text-terminal-green font-bold mb-1.5">
                  Open{parsed().open.length > 0 && <span class="text-neutral-600 ml-1 font-normal">({parsed().open.filter(t => !t.checked).length}/{parsed().open.length})</span>}
                </h4>
                <Show when={parsed().open.length > 0} fallback={<p class="text-[10px] text-neutral-600 italic">No open items</p>}>
                  <ul class="space-y-1">
                    <For each={parsed().open}>
                      {(t) => (
                        <li class="flex items-start gap-2">
                          <input
                            type="checkbox"
                            checked={t.checked}
                            onChange={() => toggleAt(t.line)}
                            class="mt-0.5 accent-terminal-green cursor-pointer"
                          />
                          <span classList={{ "line-through text-neutral-600": t.checked, "text-neutral-200": !t.checked }}>
                            {t.text}
                          </span>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </section>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
