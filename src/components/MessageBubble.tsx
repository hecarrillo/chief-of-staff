import { Show } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Message } from "../lib/types";

interface Props {
  message: Message;
  onReply?: (msg: Message) => void;
}

export default function MessageBubble(props: Props) {
  const isHector = () => props.message.from === "hector" || props.message.from === "telegram";
  const isTelegram = () => props.message.from === "telegram";
  const isSystem = () => props.message.from === "system";

  const label = () => {
    switch (props.message.from) {
      case "hector": return "you";
      case "cos": return "CoS";
      case "telegram": return "tg";
      case "system": return "sys";
    }
  };

  const time = () => {
    const d = new Date(props.message.timestamp);
    return d.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" });
  };

  const imageSrc = () => {
    const img = props.message.image;
    if (!img) return null;
    if (img.startsWith("http")) return img;
    return convertFileSrc(img);
  };

  return (
    <div class={`group flex flex-col gap-1 ${isHector() ? "items-end" : "items-start"}`}>
      <div class="flex items-center gap-2 text-[10px] text-terminal-dim">
        <span>{label()}</span>
        <span>{time()}</span>
        <Show when={isHector()}>
          <span class={props.message.delivered ? "text-blue-400" : "text-neutral-600"}>
            {props.message.delivered ? "\u2713\u2713" : "\u2713"}
          </span>
        </Show>
        <Show when={props.onReply && !isSystem()}>
          <button
            onClick={() => props.onReply?.(props.message)}
            class="opacity-0 group-hover:opacity-100 text-neutral-600 hover:text-neutral-300 transition-opacity"
          >
            reply
          </button>
        </Show>
      </div>

      <Show when={props.message.reply_preview}>
        <div class={`max-w-[80%] px-2 py-1 rounded text-[10px] border-l-2 ${
          isHector() ? "border-cos-accent/40 text-neutral-500" : "border-neutral-600 text-neutral-500"
        }`}>
          {props.message.reply_preview}
        </div>
      </Show>

      <div
        class={`max-w-[80%] rounded-lg text-sm whitespace-pre-wrap overflow-hidden ${
          isTelegram()
            ? "bg-terminal-green/20 text-terminal-green"
            : isHector()
              ? "bg-cos-accent/20 text-cos-accent"
              : isSystem()
                ? "bg-neutral-800 text-terminal-dim italic"
                : "bg-surface text-neutral-200 border border-neutral-800"
        }`}
      >
        <Show when={imageSrc()}>
          <img
            src={imageSrc()!}
            class="max-w-full max-h-64 object-contain cursor-pointer"
            onClick={() => window.open(imageSrc()!, "_blank")}
          />
        </Show>
        <Show when={props.message.text && props.message.text !== "[Photo]"}>
          <div class="px-3 py-2">{props.message.text}</div>
        </Show>
      </div>
    </div>
  );
}
