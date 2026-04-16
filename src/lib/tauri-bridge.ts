import { invoke } from "@tauri-apps/api/core";
import type { Message, Mode, SessionInfo, WindowInfo, VaultFile } from "./types";

export async function sendMessage(text: string): Promise<Message> {
  return invoke("send_message", { text });
}

export async function getMessages(): Promise<Message[]> {
  return invoke("get_messages");
}

export async function getMode(): Promise<Mode> {
  return invoke("get_mode") as Promise<Mode>;
}

export async function setMode(mode: Mode): Promise<void> {
  return invoke("set_mode_manual", { mode });
}

export async function getSessions(): Promise<SessionInfo[]> {
  return invoke("get_sessions");
}

export async function getWindows(session: string): Promise<WindowInfo[]> {
  return invoke("get_windows", { session });
}

export async function getScreen(target: string, lines: number): Promise<string> {
  return invoke("get_screen", { target, lines });
}

export async function sendToSession(target: string, text: string): Promise<void> {
  return invoke("send_to_session", { target, text });
}

export async function setTargetWindow(target: string): Promise<void> {
  return invoke("set_target_window", { target });
}

export async function getTargetWindow(): Promise<string> {
  return invoke("get_target_window");
}

export interface SessionStatus {
  exists: boolean;
  ready: boolean;
  target: string;
}

export async function getSessionStatus(): Promise<SessionStatus> {
  return invoke("get_session_status");
}

export interface SystemCheck {
  os: string;
  tmux_found: boolean;
  tmux_path: string;
  claude_found: boolean;
  config_exists: boolean;
  home_dir: string;
}

export async function checkSystem(): Promise<SystemCheck> {
  return invoke("check_system");
}

export interface BridgeConfig {
  bot_token: string;
  chat_id: string;
  http_port: number;
  vault_path: string;
  cos_session: string;
  cos_cwd: string;
  cos_framework_path: string;
  cos_framework: string;
}

export async function getConfig(): Promise<BridgeConfig> {
  return invoke("get_config");
}

export async function saveConfig(config: BridgeConfig): Promise<void> {
  return invoke("save_config", { config });
}

export async function getVaultFiles(category: string): Promise<VaultFile[]> {
  return invoke("get_vault_files", { category });
}

export async function readVaultFile(path: string): Promise<VaultFile> {
  return invoke("read_vault_file", { path });
}
