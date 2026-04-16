import { createSignal, onMount, Show, For } from "solid-js";
import { checkSystem, saveConfig } from "../lib/tauri-bridge";
import type { SystemCheck, BridgeConfig } from "../lib/tauri-bridge";
import { default_framework } from "../lib/defaults";

type Step = "checking" | "prerequisites" | "configure" | "done";

export default function Setup(props: { onComplete: () => void }) {
  const [step, setStep] = createSignal<Step>("checking");
  const [check, setCheck] = createSignal<SystemCheck | null>(null);
  const [cosDir, setCosDir] = createSignal("");
  const [sessionName, setSessionName] = createSignal("cos");
  const [httpPort, setHttpPort] = createSignal("7890");
  const [vaultPath, setVaultPath] = createSignal("");
  const [frameworkPath, setFrameworkPath] = createSignal("");
  const [botToken, setBotToken] = createSignal("");
  const [chatId, setChatId] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");

  onMount(async () => {
    try {
      const sys = await checkSystem();
      setCheck(sys);
      setCosDir(sys.home_dir);

      if (sys.config_exists) {
        // Already set up — skip wizard
        props.onComplete();
        return;
      }

      if (sys.tmux_found && sys.claude_found) {
        setStep("configure");
      } else {
        setStep("prerequisites");
      }
    } catch (e) {
      setError(String(e));
      setStep("prerequisites");
    }
  });

  async function handleSave() {
    setSaving(true);
    setError("");
    try {
      const port = parseInt(httpPort()) || 7890;
      // Replace port in framework template so curl commands match
      const framework = default_framework.replace(/localhost:7890/g, `localhost:${port}`);
      const config: BridgeConfig = {
        bot_token: botToken(),
        chat_id: chatId(),
        http_port: port,
        vault_path: vaultPath(),
        cos_session: sessionName(),
        cos_cwd: cosDir(),
        cos_framework_path: frameworkPath(),
        cos_framework: framework,
      };
      await saveConfig(config);
      setStep("done");
      // Brief pause to show success, then transition
      setTimeout(() => props.onComplete(), 1500);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  const sys = () => check();

  return (
    <div class="flex items-center justify-center h-screen bg-neutral-950">
      <div class="w-[480px] space-y-6">
        {/* Header */}
        <div class="text-center">
          <h1 class="text-lg font-bold text-terminal-green tracking-wide">CoS Desktop</h1>
          <p class="text-xs text-neutral-500 mt-1">First-time setup</p>
        </div>

        {/* Step: Checking */}
        <Show when={step() === "checking"}>
          <div class="text-center space-y-3">
            <div class="w-4 h-4 rounded-full bg-cos-accent animate-pulse mx-auto" />
            <p class="text-xs text-neutral-400">Checking system...</p>
          </div>
        </Show>

        {/* Step: Prerequisites */}
        <Show when={step() === "prerequisites"}>
          <div class="bg-surface border border-neutral-800 rounded-lg p-5 space-y-4">
            <h2 class="text-sm font-medium text-neutral-200">Prerequisites</h2>

            <div class="space-y-2">
              <CheckRow
                label="tmux"
                found={sys()?.tmux_found ?? false}
                hint={sys()?.os === "windows"
                  ? "Install via WSL: wsl --install, then sudo apt install tmux"
                  : "Install via: brew install tmux"
                }
              />
              <CheckRow
                label="Claude CLI"
                found={sys()?.claude_found ?? false}
                hint="Install: npm install -g @anthropic-ai/claude-code"
              />
            </div>

            <Show when={sys()?.tmux_found && sys()?.claude_found}>
              <button
                onClick={() => setStep("configure")}
                class="w-full py-2 bg-cos-accent/20 text-cos-accent rounded text-xs font-medium hover:bg-cos-accent/30 transition-colors"
              >
                Continue
              </button>
            </Show>

            <Show when={!(sys()?.tmux_found && sys()?.claude_found)}>
              <p class="text-[10px] text-neutral-600">
                Install the missing dependencies above, then restart the app.
              </p>
              <button
                onClick={async () => {
                  const s = await checkSystem();
                  setCheck(s);
                  if (s.tmux_found && s.claude_found) setStep("configure");
                }}
                class="w-full py-2 bg-neutral-800 text-neutral-300 rounded text-xs hover:bg-neutral-700 transition-colors"
              >
                Re-check
              </button>
            </Show>
          </div>
        </Show>

        {/* Step: Configure */}
        <Show when={step() === "configure"}>
          <div class="bg-surface border border-neutral-800 rounded-lg p-5 space-y-4">
            <h2 class="text-sm font-medium text-neutral-200">Configure</h2>

            <Field
              label="Working directory"
              value={cosDir()}
              onChange={setCosDir}
              help="Where Claude starts. Your main project directory."
            />

            <div class="grid grid-cols-2 gap-3">
              <Field
                label="Session name"
                value={sessionName()}
                onChange={setSessionName}
                help="tmux session name. Use different names for multiple CoS instances."
              />
              <Field
                label="HTTP port"
                value={httpPort()}
                onChange={setHttpPort}
                help="Bridge port. Use different ports for multiple instances."
              />
            </div>

            <Field
              label="Vault path (optional)"
              value={vaultPath()}
              onChange={setVaultPath}
              help="Obsidian vault for the Dashboard page. Leave blank to skip."
            />

            <Field
              label="Framework file (optional)"
              value={frameworkPath()}
              onChange={setFrameworkPath}
              help="Path to a .md file with your CoS framework. Leave blank to use the built-in default."
            />

            <div class="border-t border-neutral-800 pt-3">
              <p class="text-[10px] text-neutral-500 mb-2">Telegram (optional — for mobile fallback)</p>
              <div class="space-y-2">
                <Field label="Bot token" value={botToken()} onChange={setBotToken} />
                <Field label="Chat ID" value={chatId()} onChange={setChatId} />
              </div>
              <p class="text-[9px] text-neutral-600 mt-1">Each CoS instance needs its own bot token to avoid conflicts.</p>
            </div>

            <Show when={error()}>
              <p class="text-xs text-red-400">{error()}</p>
            </Show>

            <button
              onClick={handleSave}
              disabled={saving()}
              class="w-full py-2 bg-cos-accent/20 text-cos-accent rounded text-xs font-medium hover:bg-cos-accent/30 disabled:opacity-30 transition-colors"
            >
              {saving() ? "Saving..." : "Start CoS Desktop"}
            </button>
          </div>
        </Show>

        {/* Step: Done */}
        <Show when={step() === "done"}>
          <div class="text-center space-y-3">
            <div class="w-4 h-4 rounded-full bg-terminal-green mx-auto" />
            <p class="text-sm text-terminal-green font-medium">Ready</p>
            <p class="text-xs text-neutral-500">Starting CoS session...</p>
          </div>
        </Show>

        {/* OS badge */}
        <Show when={sys()}>
          <p class="text-[9px] text-neutral-700 text-center">
            {sys()!.os} detected
          </p>
        </Show>
      </div>
    </div>
  );
}

function CheckRow(props: { label: string; found: boolean; hint: string }) {
  return (
    <div class="flex items-start gap-3 py-1">
      <span class={`text-xs mt-0.5 ${props.found ? "text-terminal-green" : "text-red-400"}`}>
        {props.found ? "ok" : "x"}
      </span>
      <div class="flex-1">
        <span class="text-xs text-neutral-300">{props.label}</span>
        <Show when={!props.found}>
          <p class="text-[10px] text-neutral-500 mt-0.5">{props.hint}</p>
        </Show>
      </div>
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
