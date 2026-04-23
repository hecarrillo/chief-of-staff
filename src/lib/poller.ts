import { messages, setMessages } from "./stores";
import { setModeSignal } from "./stores";
import type { Message } from "./types";

let started = false;

/// Cheap equality: last id + length. Good enough — backend messages are append-only.
function sameMessages(a: Message[], b: Message[]): boolean {
  if (a.length !== b.length) return false;
  if (a.length === 0) return true;
  return a[a.length - 1].id === b[b.length - 1].id && a[0].id === b[0].id;
}

export function startPolling() {
  if (started) return;
  started = true;

  // Poll messages every 2 seconds — only update if content actually changed
  setInterval(async () => {
    try {
      const resp = await fetch("http://localhost:7890/api/messages");
      if (resp.ok) {
        const msgs: Message[] = await resp.json();
        if (!sameMessages(messages(), msgs)) {
          setMessages(msgs);
        }
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
