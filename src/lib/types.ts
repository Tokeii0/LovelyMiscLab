// TypeScript mirror of the Rust node model (crates/misclab-core/src/node,
// graph/port.rs). Kept in sync by hand until specta auto-generation lands.

export type PortType =
  | "any"
  | "text"
  | "number"
  | "bool"
  | "json"
  | "stringList"
  | "candidates"
  | "bytes"
  | "artifact"
  | "image"
  | "fingerprint";

export interface ScoredString {
  text: string;
  score: number;
  note?: string;
}

export type PortValue =
  | { type: "none" }
  | { type: "text"; value: string }
  | { type: "number"; value: number }
  | { type: "bool"; value: boolean }
  | { type: "json"; value: unknown }
  | { type: "stringList"; value: string[] }
  | { type: "candidates"; value: ScoredString[] }
  | { type: "bytes"; value: number[] }
  | { type: "artifact"; value: string }
  | { type: "image"; value: string }
  | { type: "fingerprint"; value: unknown };

export interface PortSpec {
  name: string;
  label: string;
  type: PortType;
  required: boolean;
  description?: string;
}

export type ParamWidget =
  | { kind: "text"; multiline: boolean }
  | { kind: "number"; min: number; max: number; step: number }
  | { kind: "slider"; min: number; max: number; step: number }
  | { kind: "select"; options: string[] }
  | { kind: "toggle" }
  | { kind: "file" };

export interface ParamSpec {
  name: string;
  label: string;
  widget: ParamWidget;
  default: unknown;
  description?: string;
}

export type Cost = "cheap" | "medium" | "heavy";

export interface NodeDescriptor {
  id: string;
  category: string;
  displayName: string;
  description?: string;
  color: string;
  inputs: PortSpec[];
  outputs: PortSpec[];
  params: ParamSpec[];
  cost: Cost;
}

/** A node as persisted/sent to the backend for execution. */
export interface SerializedNode {
  id: string;
  descriptorId: string;
  params: unknown;
  position: [number, number];
}

export interface SerializedEdge {
  from: { node: string; port: string };
  to: { node: string; port: string };
}

export interface SerializedGraph {
  nodes: SerializedNode[];
  edges: SerializedEdge[];
}

export type GraphOutputs = Record<string, Record<string, PortValue>>;

export type ProgressMsg =
  | { kind: "jobStarted"; job: string }
  | { kind: "nodeEntered"; node: string }
  | { kind: "nodeProgress"; node: string; pct: number }
  | { kind: "nodeDone"; node: string }
  | { kind: "nodeFailed"; node: string; error: string }
  | { kind: "log"; node: string | null; level: string; message: string }
  | { kind: "jobDone"; job: string }
  | { kind: "jobFailed"; job: string; error: string };
