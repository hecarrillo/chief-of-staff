import { createSignal } from "solid-js";
import type { Message, Mode } from "./types";

export const [messages, setMessages] = createSignal<Message[]>([]);
export const [mode, setModeSignal] = createSignal<Mode>("at_desk");
export const [connected, setConnected] = createSignal(true);

export function addMessage(msg: Message) {
  setMessages((prev) => {
    if (prev.some((m) => m.id === msg.id)) return prev;
    return [...prev, msg];
  });
}
