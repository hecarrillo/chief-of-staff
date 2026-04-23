import { invoke } from "@tauri-apps/api/core";

export interface DailyNote {
  date: string;
  path: string;
  content: string;
}

export async function readDailyNote(date?: string): Promise<DailyNote> {
  return invoke("read_daily_note", { date });
}

export async function writeDailyNote(content: string, date?: string): Promise<void> {
  return invoke("write_daily_note", { content, date });
}

// ---- parser ----

export interface TodoItem { line: number; text: string; checked: boolean; }
export interface LogItem { line: number; time: string; body: string; tags: string[]; }

export interface ParsedDaily {
  todos: TodoItem[];
  log: LogItem[];
  open: TodoItem[];
  /// lines before the first section header (title, etc.)
  header: string;
}

const TAG_RE = /#(decision|wiki:[\w-]+|source|research:[\w-]+|spec:[\w-]+|reading)/g;

export function parseDaily(content: string): ParsedDaily {
  const lines = content.split("\n");
  const todos: TodoItem[] = [];
  const log: LogItem[] = [];
  const open: TodoItem[] = [];
  const headerLines: string[] = [];
  let section: "header" | "todos" | "log" | "open" | "other" = "header";

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const line = raw.trimEnd();

    if (/^##\s+Todos\s*$/i.test(line)) { section = "todos"; continue; }
    if (/^##\s+Log\s*$/i.test(line)) { section = "log"; continue; }
    if (/^##\s+Open\s*$/i.test(line)) { section = "open"; continue; }
    if (/^##\s+/.test(line)) { section = "other"; continue; }

    if (section === "header") {
      headerLines.push(line);
      continue;
    }

    const todoMatch = /^\s*-\s+\[( |x)\]\s+(.+)$/i.exec(line);
    if (todoMatch) {
      const item: TodoItem = {
        line: i,
        text: todoMatch[2],
        checked: todoMatch[1].toLowerCase() === "x",
      };
      if (section === "todos") todos.push(item);
      else if (section === "open") open.push(item);
      continue;
    }

    if (section === "log" && line.trim()) {
      const timeMatch = /^(\d{1,2}:\d{2})\s*[—-]\s*(.+)$/.exec(line.trim());
      if (timeMatch) {
        const body = timeMatch[2];
        const tags = Array.from(body.matchAll(TAG_RE)).map((m) => m[0]);
        log.push({ line: i, time: timeMatch[1], body, tags });
      }
    }
  }

  return { todos, log, open, header: headerLines.join("\n").trim() };
}

/// Toggle a single todo checkbox at `line` in the given content, returns new content.
export function toggleTodoAt(content: string, lineIdx: number): string {
  const lines = content.split("\n");
  if (lineIdx < 0 || lineIdx >= lines.length) return content;
  const line = lines[lineIdx];
  const m = /^(\s*-\s+\[)([ xX])(\]\s+.+)$/.exec(line);
  if (!m) return content;
  const next = m[2].toLowerCase() === "x" ? " " : "x";
  lines[lineIdx] = `${m[1]}${next}${m[3]}`;
  return lines.join("\n");
}
