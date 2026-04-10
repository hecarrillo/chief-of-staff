import { setMessages } from "./stores";
import { setModeSignal } from "./stores";
import type { Message } from "./types";

let started = false;

export function startPolling() {
  if (started) return;
  started = true;

  // Poll messages every 2 seconds
  setInterval(async () => {
    try {
      const resp = await fetch("http://localhost:7890/api/messages");
      if (resp.ok) {
        const msgs: Message[] = await resp.json();
        setMessages(msgs);
      }
    } catch { /* bridge not ready */ }
  }, 2000);

  // Poll mode every 3 seconds
  setInterval(async () => {
    try {
      const resp = await fetch("http://localhost:7890/api/mode");
      if (resp.ok) {
        const data = await resp.json();
        setModeSignal(data.mode);
      }
    } catch { /* bridge not ready */ }
  }, 3000);
}
