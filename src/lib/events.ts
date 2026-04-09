import { listen } from "@tauri-apps/api/event";
import type { Message, VaultChange } from "./types";

export function onNewMessage(callback: (msg: Message) => void) {
  return listen<Message>("new-message", (event) => {
    callback(event.payload);
  });
}

export function onVaultChange(callback: (change: VaultChange) => void) {
  return listen<VaultChange>("vault-change", (event) => {
    callback(event.payload);
  });
}
