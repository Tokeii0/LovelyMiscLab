// Hand-written typed wrappers over Tauri commands. (tauri-specta auto-generation
// is deferred; when the command surface stabilizes we can regenerate this file.)
import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  GraphOutputs,
  NodeDescriptor,
  PortValue,
  ProgressMsg,
  SerializedGraph,
} from "./types";

export interface AppInfo {
  name: string;
  version: string;
  coreVersion: string;
}

export interface ModelConfig {
  model: string;
  apiKey: string;
  baseUrl: string;
}

export interface AiConfig {
  llm: ModelConfig;
  vision: ModelConfig;
}

export interface AppSettings {
  ai: AiConfig;
  outputDir: string;
  tools: Record<string, string>;
}

export interface ToolStatus {
  available: boolean;
  version: string;
}

export const api = {
  ping: (name: string) => invoke<string>("ping", { name }),
  appInfo: () => invoke<AppInfo>("app_info"),
  dbHealth: () => invoke<number>("db_health"),

  listNodeDescriptors: () =>
    invoke<NodeDescriptor[]>("list_node_descriptors"),

  runNode: (
    descriptorId: string,
    inputs: Record<string, PortValue>,
    params: unknown
  ) =>
    invoke<Record<string, PortValue>>("run_node", {
      descriptorId,
      inputs,
      params,
    }),

  /** Run a graph, streaming per-node progress; resolves with all node outputs. */
  runGraph: (graph: SerializedGraph, onEvent: (m: ProgressMsg) => void) => {
    const channel = new Channel<ProgressMsg>();
    channel.onmessage = onEvent;
    return invoke<GraphOutputs>("run_graph", { graph, onEvent: channel });
  },

  cancelJob: (job: string) => invoke<void>("cancel_job", { job }),
  resetRun: () => invoke<void>("reset_run"),

  getSettings: () => invoke<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) => invoke<void>("set_settings", { settings }),
  detectTool: (path: string, arg?: string) =>
    invoke<ToolStatus>("detect_tool", { path, arg }),
};
