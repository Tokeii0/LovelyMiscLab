// Hand-written typed wrappers over Tauri commands. (tauri-specta auto-generation
// is deferred; when the command surface stabilizes we can regenerate this file.)
import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  CompositeModule,
  GraphOutputs,
  NodeDescriptor,
  PortValue,
  ProgressMsg,
  ScriptModule,
  SerializedGraph,
} from "./types";

export interface AppInfo {
  name: string;
  version: string;
  coreVersion: string;
}

export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  notes: string;
  releaseUrl: string;
  downloadUrl: string;
  assetName: string;
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

export interface McpStatus {
  running: boolean;
  port: number;
  bindAll: boolean;
  endpoint: string;
}

export interface McpSettings {
  enabled: boolean;
  port: number;
  token: string | null;
  bindAll: boolean;
}

export interface GeneratedNode {
  key: string;
  descriptorId: string;
  params: Record<string, unknown>;
  position: { x: number; y: number };
}
export interface GeneratedEdge {
  from: { node: string; port: string };
  to: { node: string; port: string };
}
export interface GeneratedGraph {
  nodes: GeneratedNode[];
  edges: GeneratedEdge[];
  notes: string;
}
export interface AiTextResult {
  text: string;
}

export interface SuggestCtx {
  descriptorId: string;
  port: string;
  direction: "in" | "out";
  /** Serialized PortType, e.g. "text". */
  portType: string;
  hint?: string;
}
export interface Suggestion {
  descriptorId: string;
  reason: string;
}

/** Streamed step of the AI agent building a workflow. Nodes are referenced by
 * the agent's own `key`; the frontend maps those to real store ids. */
export type AgentEvent =
  | { kind: "started"; job: string; stepsMax: number }
  | { kind: "thinking"; text: string }
  | { kind: "addNode"; key: string; descriptorId: string; params: unknown; reason: string }
  | {
      kind: "connect";
      fromKey: string;
      fromPort: string;
      toKey: string;
      toPort: string;
      reason: string;
    }
  | { kind: "setParam"; key: string; name: string; value: unknown; reason: string }
  | { kind: "runStart"; keys: string[] }
  | { kind: "nodeResult"; key: string; ok: boolean; summary: string }
  | { kind: "toolError"; tool: string; message: string }
  | { kind: "done"; notes: string; stepsUsed: number }
  | { kind: "error"; message: string };

export const api = {
  ping: (name: string) => invoke<string>("ping", { name }),
  appInfo: () => invoke<AppInfo>("app_info"),
  dbHealth: () => invoke<number>("db_health"),

  // Self-update (portable exe): check GitHub Releases, then download + swap.
  checkUpdate: () => invoke<UpdateInfo>("check_update"),
  installUpdate: (downloadUrl: string) => invoke<void>("install_update", { downloadUrl }),

  listNodeDescriptors: () =>
    invoke<NodeDescriptor[]>("list_node_descriptors"),

  /** Run a single node silently (no progress stream). */
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

  /** Run a single node while streaming live progress/logs (stamped with `nodeId`);
   * the run is cancellable via `cancelJob`. Resolves with the node's outputs. */
  runNodeStreamed: (
    descriptorId: string,
    nodeId: string,
    inputs: Record<string, PortValue>,
    params: unknown,
    onEvent: (m: ProgressMsg) => void
  ) => {
    const channel = new Channel<ProgressMsg>();
    channel.onmessage = onEvent;
    return invoke<Record<string, PortValue>>("run_node_streamed", {
      descriptorId,
      nodeId,
      inputs,
      params,
      onEvent: channel,
    });
  },

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

  /** Ask the LLM to assemble a node graph from a natural-language task. */
  generateWorkflow: (prompt: string) =>
    invoke<GeneratedGraph>("generate_workflow", { prompt }),
  explainWorkflow: (graph: SerializedGraph, prompt: string) =>
    invoke<AiTextResult>("explain_workflow", { graph, prompt }),
  /** Suggest compatible next/previous nodes for a hovered port. */
  suggestNextNodes: (ctx: SuggestCtx) =>
    invoke<Suggestion[]>("suggest_next_nodes", { ctx }),
  /** Build a workflow step-by-step, streaming each action to the canvas. */
  agentRun: (prompt: string, onEvent: (m: AgentEvent) => void) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("agent_run", { prompt, onEvent: channel });
  },

  // User-defined composite (sub-graph) modules.
  listCompositeModules: () => invoke<CompositeModule[]>("list_composite_modules"),
  saveCompositeModule: (module: CompositeModule) =>
    invoke<void>("save_composite_module", { module }),
  deleteCompositeModule: (id: string) =>
    invoke<void>("delete_composite_module", { id }),

  // User-defined script/program nodes.
  listScriptModules: () => invoke<ScriptModule[]>("list_script_modules"),
  saveScriptModule: (module: ScriptModule) =>
    invoke<void>("save_script_module", { module }),
  deleteScriptModule: (id: string) =>
    invoke<void>("delete_script_module", { id }),

  // Flow project files.
  saveProject: (path: string, contents: string) =>
    invoke<void>("save_project", { path, contents }),
  loadProject: (path: string) => invoke<string>("load_project", { path }),

  // Embedded MCP server (only present when the app is built with `--features mcp`).
  syncCanvas: (snapshot: unknown) => invoke<void>("sync_canvas", { snapshot }),
  mcpStatus: () => invoke<McpStatus>("mcp_status"),
  mcpStart: () => invoke<McpStatus>("mcp_start"),
  mcpStop: () => invoke<McpStatus>("mcp_stop"),
  mcpGetConfig: () => invoke<McpSettings>("mcp_get_config"),
  mcpSetConfig: (config: McpSettings) => invoke<void>("mcp_set_config", { config }),
};
