import { createSignal, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export interface QuestionPayload {
  id: string;
  question: string;
  options: string[];
  multi_select: boolean;
}

interface Props {
  question: QuestionPayload;
  onAnswered: () => void;
}

export default function QuestionCard(props: Props) {
  const [answered, setAnswered] = createSignal(false);
  const [selected, setSelected] = createSignal<string | null>(null);

  async function answer(option: string) {
    if (answered()) return;
    setSelected(option);
    setAnswered(true);
    await invoke("answer_question", { id: props.question.id, selected: [option] });
    props.onAnswered();
  }

  return (
    <div class="bg-cos-accent/10 border border-cos-accent/30 rounded-lg p-3 space-y-2">
      <p class="text-xs text-cos-accent font-medium">{props.question.question}</p>
      <div class="flex flex-wrap gap-1.5">
        <For each={props.question.options}>
          {(opt, i) => (
            <button
              onClick={() => answer(opt)}
              disabled={answered()}
              class={`px-3 py-1.5 rounded text-xs transition-colors ${
                selected() === opt
                  ? "bg-cos-accent text-neutral-950 font-medium"
                  : answered()
                    ? "bg-neutral-800 text-neutral-600 cursor-not-allowed"
                    : "bg-neutral-800 text-neutral-200 hover:bg-neutral-700"
              }`}
            >
              {i() + 1}. {opt}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}
