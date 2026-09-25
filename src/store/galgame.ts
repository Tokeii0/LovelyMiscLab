import { create } from "zustand";

import {
  api,
  type GalgameChoice,
  type GalgameHistoryItem,
  type GalgameStepRequest,
  type GalgameTurn,
} from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { errorCode, errorMessage, isCancelled } from "@/lib/errors";

/** Sprite/background moods the narrator LLM may return. */
export type Mood = "neutral" | "happy" | "thinking" | "worried" | "excited";

interface GalgameState {
  /** false ⇒ show the challenge-input intro screen. */
  started: boolean;
  /** A step (narration / engine action) is in flight. */
  busy: boolean;
  error: string;
  /** Backend error code of `error` (e.g. "ai_config" → offer a Settings link). */
  errorCode: string | null;

  challenge: string;
  challengeKind: string;
  /** Optional problem statement / hint typed with a file/image challenge (题干);
   * echoed to the engine every round for context. */
  brief: string;
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
  /** The working data is an encrypted archive; free text is tried as its password. */
  awaitingPassword: boolean;

  /** `kind`: "text" | "image" | "file". `brief`: optional 题干 for a file/image. */
  start: (challenge: string, kind?: string, brief?: string) => void;
  pick: (choice: GalgameChoice) => void;
  /** Re-send the step that just failed. */
  retry: () => void;
  /** Stop the step in flight (cancels a running tool); the scene stays as it was. */
  cancel: () => void;
  /** Back to the intro screen (keeps nothing). */
  reset: () => void;
}

const base = {
  started: false,
  busy: false,
  error: "",
  errorCode: null as string | null,
  challenge: "",
  challengeKind: "text",
  brief: "",
  workData: "",
  speaker: "Misca",
  mood: "neutral",
  narration: "",
  choices: [] as GalgameChoice[],
  history: [] as GalgameHistoryItem[],
  lastOutputs: null as string | null,
  ending: null as string | null,
  sceneSeed: 0,
  awaitingPassword: false,
};

// Each request gets an id; a reply for anything but the latest is dropped, so a
// slow answer can't land in a restarted story or after 停止.
let reqSeq = 0;
let currentJob: string | null = null;
let lastRequest: { req: GalgameStepRequest; history: GalgameHistoryItem[] } | null = null;
/** How the scene looked before the step in flight (restored on error / 停止). */
let beforeStep: Pick<GalgameState, "lastOutputs" | "history"> | null = null;

/** Canned scene for the browser dev preview (no Tauri IPC / real engine). */
function mockTurn(req: GalgameStepRequest): GalgameTurn {
  const solving = !!req.picked?.node;
  return {
    speaker: "Misca",
    mood: solving ? "excited" : "thinking",
    narration: req.picked
      ? `（预览）「${req.picked.text}」……${
          solving
            ? "哼，早把数据喂给工具跑好了。浏览器预览只是模拟数据啦，装进桌面应用才是真本事——才、才不是特意帮你。"
            : "急什么，再多观察观察不行吗。"
        }`
      : "（预览剧情）哼，这种数据也想难住本小姐？浏览器预览用的是模拟数据；装进桌面应用、在「设置」里配好 AI 文本模型，我才好真刀真枪帮你解——别、别误会，我只是顺手而已。",
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
    const id = ++reqSeq;
    lastRequest = { req, history };
    // Show "running" on the result panel, but keep what to restore if it fails.
    const before = { lastOutputs: get().lastOutputs, history: get().history };
    beforeStep = before;
    set({
      busy: true,
      error: "",
      errorCode: null,
      ...(req.picked?.node
        ? { lastOutputs: `正在执行：${req.picked.text}\n\n工具：${req.picked.node}\n等待节点结果与 Misca 解读…` }
        : {}),
    });
    try {
      let turn: GalgameTurn;
      if (inTauri) {
        turn = await api.galgameStep(req, (job) => {
          if (id === reqSeq) currentJob = job;
        });
      } else {
        await new Promise((r) => setTimeout(r, 400));
        turn = mockTurn(req);
      }
      if (id !== reqSeq) return;
      set((s) => ({
        busy: false,
        history,
        speaker: turn.speaker || "Misca",
        mood: turn.mood || "neutral",
        narration: turn.narration || "",
        choices: turn.choices ?? [],
        lastOutputs: turn.outputs ?? null,
        ending: turn.ending ?? null,
        awaitingPassword: !!turn.awaitingPassword,
        sceneSeed: s.sceneSeed + 1,
        workData: turn.resultData ?? s.workData,
        // Once a round yields text, the chain is text from here on; a data-URL
        // result (a produced file/image) stays binary.
        challengeKind:
          turn.resultData != null
            ? turn.resultData.startsWith("data:")
              ? s.challengeKind
              : "text"
            : s.challengeKind,
      }));
    } catch (e) {
      if (id !== reqSeq) return;
      // Nothing happened: undo the placeholder and keep the scene as it was.
      set({
        busy: false,
        error: isCancelled(e) ? "" : errorMessage(e),
        errorCode: errorCode(e),
        ...before,
      });
    } finally {
      if (id === reqSeq) currentJob = null;
    }
  };

  return {
    ...base,

    start: (challenge, kind = "text", brief = "") => {
      const c = challenge.trim();
      const b = brief.trim();
      set({ ...base, started: true, challenge: c, workData: c, challengeKind: kind, brief: b });
      void run({ challenge: c, challengeKind: kind, brief: b, history: [], picked: null }, []);
    },

    pick: (choice) => {
      const st = get();
      if (st.busy) return;
      // The history entry only becomes real once the step succeeds (see run).
      const history = [
        ...st.history,
        { narration: st.narration, picked: choice.text, outputs: st.lastOutputs ?? null },
      ];
      void run(
        {
          challenge: st.workData,
          challengeKind: st.challengeKind,
          brief: st.brief,
          history,
          picked: { text: choice.text, node: choice.node ?? null, params: choice.params ?? null },
          awaitingPassword: st.awaitingPassword,
        },
        history
      );
    },

    retry: () => {
      if (get().busy || !lastRequest) return;
      void run(lastRequest.req, lastRequest.history);
    },

    cancel: () => {
      if (!get().busy) return;
      reqSeq++; // drop whatever comes back
      if (currentJob && inTauri) void api.cancelJob(currentJob).catch(() => {});
      currentJob = null;
      set({ busy: false, error: "", ...(beforeStep ?? {}) });
    },

    reset: () => {
      get().cancel();
      lastRequest = null;
      set({ ...base });
    },
  };
});
