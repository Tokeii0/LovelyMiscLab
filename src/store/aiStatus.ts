import { create } from "zustand";

import { api, type AppSettings } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";

/** Whether the AI models are set up, known up front so AI features can point to
 * Settings instead of failing on the first request. null = not loaded yet. */
interface AiStatusState {
  llm: boolean | null;
  vision: boolean | null;
  set: (settings: AppSettings) => void;
  load: () => Promise<void>;
}

const ready = (m: { baseUrl: string; model: string }) => !!m.baseUrl.trim() && !!m.model.trim();

export const useAiStatus = create<AiStatusState>((set) => ({
  llm: null,
  vision: null,
  set: (s) => set({ llm: ready(s.ai.llm), vision: ready(s.ai.vision) }),
  load: async () => {
    if (!inTauri) return;
    try {
      const s = await api.getSettings();
      set({ llm: ready(s.ai.llm), vision: ready(s.ai.vision) });
    } catch {
      /* leave unknown; the call itself will report */
    }
  },
}));
