import { createSignal, onMount, Show } from "solid-js";
import { getConfig, saveConfig } from "../lib/tauri-bridge";
import type { BridgeConfig } from "../lib/tauri-bridge";

export default function Settings() {
  const [config, setConfig] = createSignal<BridgeConfig | null>(null);
  const [saved, setSaved] = createSignal(false);
  const [error, setError] = createSignal("");

  onMount(async () => {
    try {
      setConfig(await getConfig());
    } catch (e) {
      setError(String(e));
    }
  });

  function update(key: keyof BridgeConfig, value: string | number) {
    setConfig((prev) => prev ? { ...prev, [key]: value } : prev);
    setSaved(false);
  }

  async function handleSave() {
    const c = config();
    if (!c) return;
    try {
      await saveConfig(c);
      setSaved(true);
      setError("");
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div class="h-full overflow-y-auto p-4 space-y-6">
      <h2 class="text-sm font-bold text-terminal-green">Settings</h2>

      <Show when={config()} fallback={<p class="text-xs text-neutral-600">Loading...</p>}>
        {/* General */}
        <Section title="Session">
          <Field
            label="Session name"
            value={config()!.cos_session}
            onChange={(v) => update("cos_session", v)}
            help="tmux session name created on startup"
          />
          <Field
            label="Working directory"
            value={config()!.cos_cwd}
            onChange={(v) => update("cos_cwd", v)}
            help="Directory where Claude starts"
          />
        </Section>

        <Section title="Vault">
          <Field
            label="Vault path"
            value={config()!.vault_path}
            onChange={(v) => update("vault_path", v)}
            help="Obsidian vault path for Dashboard"
          />
        </Section>

        <Section title="Telegram (optional)">
          <Field
            label="Bot token"
            value={config()!.bot_token}
            onChange={(v) => update("bot_token", v)}
          />
          <Field
            label="Chat ID"
            value={config()!.chat_id}
            onChange={(v) => update("chat_id", v)}
          />
        </Section>

        <Section title="Bridge">
          <Field
            label="HTTP port"
            value={String(config()!.http_port)}
            onChange={(v) => update("http_port", parseInt(v) || 7890)}
          />
        </Section>

        {/* Framework */}
        <Section title="CoS Framework">
          <Field
            label="Framework file path (optional)"
            value={config()!.cos_framework_path}
            onChange={(v) => update("cos_framework_path", v)}
            help="Path to an external framework file (e.g. Obsidian vault). Takes priority over the inline editor below."
          />
          <Show when={config()!.cos_framework_path}>
            <p class="text-[10px] text-cos-accent mt-1">
              Using external file. Inline editor below is the fallback if the file is unreadable.
            </p>
          </Show>
          <p class="text-[10px] text-neutral-500 mb-2 mt-3">
            {config()!.cos_framework_path
              ? "Fallback framework (used when external file is unreadable):"
              : "System prompt sent to Claude on session startup:"}
          </p>
          <textarea
            value={config()!.cos_framework}
            onInput={(e) => update("cos_framework", e.currentTarget.value)}
            class="w-full h-80 bg-neutral-900 border border-neutral-700 rounded-lg px-3 py-2 text-[11px] text-neutral-200 font-mono resize-y focus:outline-none focus:border-cos-accent"
          />
        </Section>

        {/* Save button */}
        <div class="flex items-center gap-3 pt-2 pb-8">
          <button
            onClick={handleSave}
            class="px-4 py-2 bg-cos-accent/20 text-cos-accent rounded-lg text-xs font-medium hover:bg-cos-accent/30 transition-colors"
          >
            Save Settings
          </button>
          <Show when={saved()}>
            <span class="text-xs text-terminal-green">Saved</span>
          </Show>
          <Show when={error()}>
            <span class="text-xs text-red-400">{error()}</span>
          </Show>
          <p class="text-[10px] text-neutral-600 ml-auto">
            Restart app for session changes to take effect
          </p>
        </div>
      </Show>
    </div>
  );
}

function Section(props: { title: string; children: any }) {
  return (
    <div>
      <h3 class="text-xs font-medium text-neutral-400 mb-2 uppercase tracking-wider">
        {props.title}
      </h3>
      <div class="space-y-3 pl-1">{props.children}</div>
    </div>
  );
}

function Field(props: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  help?: string;
}) {
  return (
    <div>
      <label class="text-[11px] text-neutral-400 block mb-1">{props.label}</label>
      <input
        type="text"
        value={props.value}
        onInput={(e) => props.onChange(e.currentTarget.value)}
        class="w-full bg-neutral-900 border border-neutral-700 rounded px-2 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-cos-accent"
      />
      <Show when={props.help}>
        <p class="text-[10px] text-neutral-600 mt-0.5">{props.help}</p>
      </Show>
    </div>
  );
}
