import { createSignal, Show } from "solid-js";
import type { VaultFile } from "../lib/types";

interface Props {
  file: VaultFile;
}

export default function ProjectCard(props: Props) {
  const [expanded, setExpanded] = createSignal(false);

  const title = () => props.file.frontmatter["title"] || cleanName(props.file.name);
  const status = () => props.file.frontmatter["status"] || "";
  const date = () => props.file.name.match(/^\d{4}-\d{2}-\d{2}/)?.[0] || "";

  return (
    <div class="border border-neutral-800 rounded-lg overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded())}
        class="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-surface-hover transition-colors"
      >
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-xs text-neutral-200 font-medium truncate">{title()}</span>
            <Show when={status()}>
              <span class={`px-1.5 py-0.5 rounded text-[9px] font-medium ${statusColor(status())}`}>
                {status()}
              </span>
            </Show>
          </div>
          <Show when={date()}>
            <span class="text-[10px] text-neutral-600 mt-0.5">{date()}</span>
          </Show>
        </div>
        <span class="text-neutral-600 text-[10px] shrink-0">{expanded() ? "v" : ">"}</span>
      </button>
      <Show when={expanded()}>
        <div class="border-t border-neutral-800 px-4 py-3 max-h-80 overflow-y-auto">
          <pre class="text-[11px] text-neutral-400 whitespace-pre-wrap font-mono leading-relaxed">
            {props.file.body.slice(0, 3000)}
            {props.file.body.length > 3000 ? "\n\n... (truncated)" : ""}
          </pre>
        </div>
      </Show>
    </div>
  );
}

function statusColor(status: string): string {
  const s = status.toLowerCase();
  if (s.includes("done") || s.includes("shipped")) return "bg-terminal-green/20 text-terminal-green";
  if (s.includes("active") || s.includes("progress")) return "bg-cos-accent/20 text-cos-accent";
  if (s.includes("blocked")) return "bg-red-500/20 text-red-400";
  return "bg-neutral-700/50 text-neutral-400";
}

function cleanName(name: string): string {
  return name.replace(/^\d{4}-\d{2}-\d{2}-?/, "").replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
