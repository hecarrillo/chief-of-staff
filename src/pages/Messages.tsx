import { createSignal, createEffect, For, onMount, onCleanup, Show } from "solid-js";
import { messages, addMessage, setMessages } from "../lib/stores";
import { sendMessage, getMessages, getSessions, getWindows, setTargetWindow, getTargetWindow, getSessionStatus } from "../lib/tauri-bridge";
import type { SessionInfo, WindowInfo } from "../lib/types";
import { onNewMessage } from "../lib/events";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import MessageBubble from "../components/MessageBubble";
import QuestionCard from "../components/QuestionCard";
import type { QuestionPayload } from "../components/QuestionCard";
import type { Message } from "../lib/types";

export default function Messages() {
  const [sending, setSending] = createSignal(false);
  const [pendingImage, setPendingImage] = createSignal<string | null>(null);
  const [replyTo, setReplyTo] = createSignal<Message | null>(null);
  const [questions, setQuestions] = createSignal<QuestionPayload[]>([]);
  const [sessions, setSessions] = createSignal<SessionInfo[]>([]);
  const [windows, setWindows] = createSignal<WindowInfo[]>([]);
  const [activeTarget, setActiveTarget] = createSignal("");
  const [activeSession, setActiveSession] = createSignal("");
  const [sessionReady, setSessionReady] = createSignal(false);
  const [statusMessage, setStatusMessage] = createSignal("Starting session...");
  let messagesEnd: HTMLDivElement | undefined;
  let inputRef!: HTMLTextAreaElement;
  let fileInputRef!: HTMLInputElement;

  async function loadSessions() {
    try {
      const list = await getSessions();
      setSessions(list);
      const current = await getTargetWindow();
      setActiveTarget(current);
      // Extract session name from target (e.g. "cos" from "cos:0")
      const sess = current.split(":")[0] || (list[0]?.name ?? "");
      setActiveSession(sess);
      if (sess) {
        const wins = await getWindows(sess);
        setWindows(wins);
      }
    } catch (e) {
      console.error("Failed to load sessions:", e);
    }
  }

  async function handleSessionChange(session: string) {
    setActiveSession(session);
    try {
      const wins = await getWindows(session);
      setWindows(wins);
      // Use just session name if single window, otherwise session:window
      const target = wins.length <= 1 ? session : `${session}:${wins[0].name}`;
      setActiveTarget(target);
      await setTargetWindow(target);
    } catch {
      setWindows([]);
    }
  }

  async function handleWindowChange(windowName: string) {
    const target = `${activeSession()}:${windowName}`;
    setActiveTarget(target);
    await setTargetWindow(target);
  }

  async function pollSessionReady() {
    const maxAttempts = 30; // 30 seconds
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const status = await getSessionStatus();
        if (!status.exists) {
          setStatusMessage("Creating tmux session...");
        } else if (!status.ready) {
          setStatusMessage("Waiting for Claude Code to start...");
        } else {
          setSessionReady(true);
          return;
        }
      } catch {
        setStatusMessage("Connecting...");
      }
      await new Promise((r) => setTimeout(r, 1000));
    }
    // After timeout, let user interact anyway
    setSessionReady(true);
  }

  // Auto-scroll when messages change (from global poller)
  createEffect(() => {
    const _len = messages().length;
    scrollToBottom();
  });

  onMount(async () => {
    try {
      const history = await getMessages();
      setMessages(history);
    } catch (e) {
      console.error("Failed to load history:", e);
    }
    loadSessions();
    pollSessionReady();
    scrollToBottom();

    const unlisten = await onNewMessage((msg) => {
      addMessage(msg);
      scrollToBottom();
    });

    const unlistenQ = await listen<QuestionPayload>("cos-question", (event) => {
      setQuestions((prev) => [...prev, event.payload]);
      scrollToBottom();
    });

    onCleanup(() => { unlisten(); unlistenQ(); });
  });

  function scrollToBottom() {
    requestAnimationFrame(() => {
      messagesEnd?.scrollIntoView({ behavior: "smooth" });
    });
  }

  async function saveImageBytes(bytes: ArrayBuffer, ext: string): Promise<string> {
    const arr = Array.from(new Uint8Array(bytes));
    return invoke("save_image", { bytes: arr, ext });
  }

  async function handlePaste(e: ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (const item of items) {
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const blob = item.getAsFile();
        if (!blob) continue;
        const ext = blob.type.split("/")[1] || "png";
        const bytes = await blob.arrayBuffer();
        const path = await saveImageBytes(bytes, ext);
        setPendingImage(path);
        return;
      }
    }
  }

  async function handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const ext = file.name.split(".").pop() || "png";
    const bytes = await file.arrayBuffer();
    const path = await saveImageBytes(bytes, ext);
    setPendingImage(path);
    input.value = "";
  }

  function autoResize() {
    inputRef.style.height = "auto";
    inputRef.style.height = Math.min(inputRef.scrollHeight, 150) + "px";
  }

  async function handleSend() {
    const text = inputRef.value.trim();
    const image = pendingImage();
    const reply = replyTo();
    if (!text && !image) return;
    if (sending()) return;

    let fullText = text;
    if (reply) {
      const preview = reply.text.slice(0, 80);
      fullText = `[Replying to: "${preview}"]\n${text}`;
    }

    setSending(true);
    try {
      if (image) {
        const msg = await invoke("send_message_with_image", {
          text: fullText || "[Photo]",
          imagePath: image,
        }) as Message;
        addMessage(msg);
      } else {
        const msg = await sendMessage(fullText);
        addMessage(msg);
      }
    } catch (err) {
      console.error("Send failed:", err);
    } finally {
      inputRef.value = "";
      inputRef.style.height = "auto";
      setPendingImage(null);
      setReplyTo(null);
      scrollToBottom();
      inputRef.focus();
      setSending(false);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
    if (e.key === "Escape") {
      setReplyTo(null);
      setPendingImage(null);
    }
  }

  function handleReply(msg: Message) {
    setReplyTo(msg);
    inputRef?.focus();
  }

  return (
    <div class="flex flex-col h-full">
      {/* Session + Window selector */}
      <div class="flex items-center gap-2 px-4 py-2 border-b border-neutral-800 bg-neutral-950/50">
        <span class="text-[10px] text-neutral-500 uppercase tracking-wider">Target</span>
        <select
          class="bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-cos-accent"
          value={activeSession()}
          onChange={(e) => handleSessionChange(e.currentTarget.value)}
        >
          <For each={sessions()}>
            {(s) => <option value={s.name}>{s.name}</option>}
          </For>
        </select>
        <Show when={windows().length > 1}>
          <span class="text-neutral-700">:</span>
          <select
            class="bg-neutral-900 border border-neutral-700 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-cos-accent"
            value={activeTarget().split(":")[1] || ""}
            onChange={(e) => handleWindowChange(e.currentTarget.value)}
          >
            <For each={windows()}>
              {(w) => <option value={w.name}>{w.name}</option>}
            </For>
          </select>
        </Show>
        <button
          onClick={loadSessions}
          class="text-[10px] text-neutral-500 hover:text-neutral-300 transition-colors px-1"
          title="Refresh sessions"
        >
          ↻
        </button>
        <span class="text-[10px] text-neutral-600 ml-auto">{activeTarget()}</span>
      </div>

      <div class="flex-1 overflow-y-auto p-4 space-y-3 relative">
        <Show when={!sessionReady()}>
          <div class="absolute inset-0 flex flex-col items-center justify-center bg-neutral-950/80 z-10">
            <div class="flex items-center gap-3 mb-3">
              <div class="w-3 h-3 rounded-full bg-cos-accent animate-pulse" />
              <span class="text-sm text-neutral-300">Initializing</span>
            </div>
            <p class="text-xs text-neutral-500">{statusMessage()}</p>
          </div>
        </Show>
        {messages().length === 0 && sessionReady() && (
          <div class="flex items-center justify-center h-full text-terminal-dim text-xs">
            No messages yet. Type below to talk to CoS.
          </div>
        )}
        <For each={messages()}>
          {(msg) => <MessageBubble message={msg} onReply={handleReply} />}
        </For>
        <For each={questions()}>
          {(q) => (
            <QuestionCard
              question={q}
              onAnswered={() => setQuestions((prev) => prev.filter((p) => p.id !== q.id))}
            />
          )}
        </For>
        <div ref={messagesEnd} />
      </div>

      <div class="border-t border-neutral-800 p-3">
        <Show when={replyTo()}>
          <div class="mb-2 flex items-center gap-2 px-2 py-1.5 bg-neutral-900 rounded border-l-2 border-cos-accent">
            <span class="text-[10px] text-neutral-400 flex-1 truncate">
              Replying to: {replyTo()!.text.slice(0, 60)}
            </span>
            <button onClick={() => setReplyTo(null)} class="text-[10px] text-neutral-600 hover:text-neutral-300">x</button>
          </div>
        </Show>
        <Show when={pendingImage()}>
          <div class="mb-2 flex items-center gap-2 px-2 py-1 bg-neutral-900 rounded border border-neutral-700">
            <span class="text-[10px] text-terminal-green">Image attached</span>
            <button onClick={() => setPendingImage(null)} class="text-[10px] text-red-400 hover:text-red-300">remove</button>
          </div>
        </Show>
        <div class="flex gap-2 items-end">
          <button
            onClick={() => fileInputRef.click()}
            class="px-2 py-2 text-terminal-dim hover:text-neutral-300 transition-colors"
            title="Attach image"
          >+</button>
          <input ref={fileInputRef} type="file" accept="image/*" class="hidden" onChange={handleFileSelect} />
          <textarea
            ref={inputRef}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            onInput={autoResize}
            placeholder="Message CoS..."
            rows={1}
            class="flex-1 bg-neutral-900 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-100 placeholder-neutral-600 resize-none focus:outline-none focus:border-cos-accent overflow-y-auto"
            style={{ "max-height": "150px" }}
          />
          <button
            onClick={handleSend}
            disabled={sending()}
            class="px-4 py-2 bg-cos-accent/20 text-cos-accent rounded-lg text-xs font-medium hover:bg-cos-accent/30 disabled:opacity-30 transition-colors"
          >
            {sending() ? "..." : "Send"}
          </button>
        </div>
        <p class="text-[10px] text-neutral-600 mt-1.5">Enter to send, Shift+Enter for newline, Esc to cancel reply</p>
      </div>
    </div>
  );
}
