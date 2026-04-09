import { createSignal, For, onMount, onCleanup, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Todo {
  id: string;
  text: string;
  done: boolean;
  date: string;
  added_by: string;
}

export default function TodoPanel() {
  const [todos, setTodos] = createSignal<Todo[]>([]);
  const [input, setInput] = createSignal("");
  let inputRef!: HTMLInputElement;

  async function load() {
    try {
      const items: Todo[] = await invoke("todo_list");
      setTodos(items);
    } catch {
      setTodos([]);
    }
  }

  onMount(async () => {
    await load();
    const unlisten = await listen<Todo[]>("todo-changed", (event) => {
      setTodos(event.payload);
    });
    onCleanup(() => unlisten());
  });

  async function addTodo() {
    const text = input().trim();
    if (!text) return;
    await invoke("todo_add", { text });
    setInput("");
    inputRef?.focus();
  }

  async function toggle(id: string) {
    await invoke("todo_toggle", { id });
  }

  async function remove(id: string) {
    await invoke("todo_remove", { id });
  }

  const done = () => todos().filter((t) => t.done).length;

  return (
    <div class="flex flex-col h-full border-l border-neutral-800 bg-surface">
      <div class="px-3 py-3 border-b border-neutral-800">
        <div class="flex items-center justify-between">
          <h3 class="text-xs font-bold text-terminal-green">Today</h3>
          <span class="text-[10px] text-neutral-600">{done()}/{todos().length}</span>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto px-2 py-2 space-y-1">
        <Show
          when={todos().length > 0}
          fallback={<p class="text-[10px] text-neutral-600 text-center py-4">No tasks yet</p>}
        >
          <For each={todos()}>
            {(todo) => (
              <div class="flex items-start gap-2 group px-1 py-1 rounded hover:bg-surface-hover">
                <button
                  onClick={() => toggle(todo.id)}
                  class={`mt-0.5 w-3.5 h-3.5 rounded border flex-shrink-0 flex items-center justify-center text-[8px] ${
                    todo.done
                      ? "bg-terminal-green/30 border-terminal-green text-terminal-green"
                      : "border-neutral-600 hover:border-neutral-400"
                  }`}
                >
                  {todo.done ? "x" : ""}
                </button>
                <span
                  class={`text-[11px] flex-1 leading-tight ${
                    todo.done ? "line-through text-neutral-600" : "text-neutral-300"
                  }`}
                >
                  {todo.text}
                </span>
                <button
                  onClick={() => remove(todo.id)}
                  class="text-[9px] text-neutral-700 hover:text-red-400 opacity-0 group-hover:opacity-100"
                >
                  x
                </button>
              </div>
            )}
          </For>
        </Show>
      </div>

      <div class="px-2 py-2 border-t border-neutral-800">
        <div class="flex gap-1">
          <input
            ref={inputRef}
            value={input()}
            onInput={(e) => setInput(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && addTodo()}
            placeholder="Add task..."
            class="flex-1 bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-[11px] text-neutral-100 placeholder-neutral-600 focus:outline-none focus:border-terminal-green"
          />
        </div>
      </div>
    </div>
  );
}
