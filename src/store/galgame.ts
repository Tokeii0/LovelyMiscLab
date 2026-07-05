import { create } from "zustand";

import {
  api,
  type GalgameChoice,
  type GalgameHistoryItem,
  type GalgameStepRequest,
  type GalgameTurn,
} from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";

/** Sprite/background moods the narrator LLM may return. */
export type Mood = "neutral" | "happy" | "thinking" | "worried" | "excited";

interface GalgameState {
  /** false ⇒ show the challenge-input intro screen. */
  started: boolean;
  /** A step (narration / engine action) is in flight. */
  busy: boolean;
  error: string;

  challenge: string;
  challengeKind: string;
  /** The evolving working data — starts as `challenge`, then advances to each
   * solving round's output so the next round continues where the last ended. */
  workData: string;

  speaker: string;
  mood: string;
  narration: string;
  choices: GalgameChoice[];
  history: GalgameHistoryItem[];
  /** Tool result summary for the current scene, if any. */
  lastOutputs: string | null;
  /** "good" | "bad" | null. */
  ending: string | null;
  /** Increments each turn; seeds the random sprite-variant pick. */
  sceneSeed: number;

  start: (challenge: string) => void;
  pick: (choice: GalgameChoice) => void;
  /** Back to the intro screen (keeps nothing). */
  reset: () => void;
}

const base = {
  started: false,
  busy: false,
  error: "",
  challenge: "",
  challengeKind: "text",
  workData: "",
  speaker: "Misca",
  mood: "neutral",
  narration: "",
  choices: [] as GalgameChoice[],
  history: [] as GalgameHistoryItem[],
  lastOutputs: null as string | null,
  ending: null as string | null,
  sceneSeed: 0,
};

/** Canned scene for the browser dev preview (no Tauri IPC / real engine). */
function mockTurn(req: GalgameStepRequest): GalgameTurn {
  const solving = !!req.picked?.node;
  return {
    speaker: "Misca",
    mood: solving ? "excited" : "thinking",
    narration: req.picked
      ? `（预览）你选了「${req.picked.text}」。${
          solving
            ? "我把当前数据直接喂给这个工具跑了一下——浏览器预览是模拟数据，装进桌面应用后就是真结果。"
            : "嗯……我们再观察观察。"
        }`
      : "（预览剧情）唔，这段数据看着可疑。浏览器预览用的是模拟数据；装进桌面应用、并在「设置」里配好 AI 文本模型后，我就能真判断内容、直接从工具库里挑对口的工具解题啦。",
    choices: [
      { text: "这串八成是 base64，解一层", node: "base64_decode" },
      { text: "当成十六进制还原", node: "hex_decode" },
      { text: "先观察一下有没有特征" },
    ],
    outputs: solving ? "（模拟）[Base64 解码] text → 下一层数据……" : null,
    ending: solving ? "good" : null,
    resultData: solving ? "（模拟）下一层解出的数据" : null,
  };
}

/** Drives 故事模式: each turn optionally runs a real solving action, then the
 * LLM narrates the next scene and offers choices. */
export const useGalgameStore = create<GalgameState>((set, get) => {
  const run = async (req: GalgameStepRequest, history: GalgameHistoryItem[]) => {
    set({ busy: true, error: "", history });
    try {
      let turn: GalgameTurn;
      if (inTauri) {
        turn = await api.galgameStep(req);
      } else {
        await new Promise((r) => setTimeout(r, 400));
        turn = mockTurn(req);
      }
      set((s) => ({
        busy: false,
        speaker: turn.speaker || "Misca",
        mood: turn.mood || "neutral",
        narration: turn.narration || "",
        choices: turn.choices ?? [],
        lastOutputs: turn.outputs ?? null,
        ending: turn.ending ?? null,
        sceneSeed: s.sceneSeed + 1,
        workData: turn.resultData ?? s.workData,
      }));
    } catch (e) {
      set({ busy: false, error: String(e) });
    }
  };

  return {
    ...base,

    start: (challenge) => {
      const c = challenge.trim();
      set({ ...base, started: true, challenge: c, workData: c });
      void run({ challenge: c, challengeKind: "text", history: [], picked: null }, []);
    },

    pick: (choice) => {
      const st = get();
      if (st.busy) return;
      const item: GalgameHistoryItem = {
        narration: st.narration,
        picked: choice.text,
        outputs: st.lastOutputs ?? null,
      };
      const history = [...st.history, item];
      if (choice.node) {
        set({ lastOutputs: `正在执行：${choice.text}\n\n工具：${choice.node}\n等待节点结果与 Misca 解读…` });
      }
      void run(
        {
          challenge: st.workData,
          challengeKind: st.challengeKind,
          history,
          picked: { text: choice.text, node: choice.node ?? null, params: choice.params ?? null },
        },
        history
      );
    },

    reset: () => set({ ...base }),
  };
});
