import { beforeEach, describe, expect, it, vi } from "vitest";

import type { NodeDescriptor } from "@/lib/types";
import { useGraphStore } from "@/store/graph";

const desc = (id: string): NodeDescriptor => ({
  id,
  category: "测试",
  displayName: id,
  color: "#888",
  inputs: [{ name: "text", label: "文本", type: "text", required: true }],
  outputs: [{ name: "text", label: "文本", type: "text", required: false }],
  params: [{ name: "key", label: "密钥", widget: { kind: "text", multiline: false }, default: "" }],
  cost: "cheap",
});

const g = () => useGraphStore.getState();

beforeEach(() => {
  vi.useFakeTimers();
  g().loadFlow([], []);
  vi.advanceTimersByTime(5000);
});

describe("undo history", () => {
  it("records a whole drag as one step", () => {
    const id = g().addNode(desc("a"), { x: 0, y: 0 });
    const steps = g().past.length;
    for (let i = 1; i <= 5; i++) {
      g().onNodesChange([{ id, type: "position", position: { x: i * 10, y: 0 }, dragging: true }]);
    }
    g().onNodesChange([{ id, type: "position", position: { x: 50, y: 0 }, dragging: false }]);
    expect(g().past.length).toBe(steps + 1);
    g().undo();
    expect(g().nodes[0].position).toEqual({ x: 0, y: 0 });
  });

  it("merges a typing burst into one step but separates distinct fields", () => {
    const id = g().addNode(desc("a"), { x: 0, y: 0 });
    const steps = g().past.length;
    for (const v of ["a", "ab", "abc"]) {
      g().setParam(id, "key", v);
      vi.advanceTimersByTime(100);
    }
    expect(g().past.length).toBe(steps + 1);
    vi.advanceTimersByTime(2000);
    g().setParam(id, "key", "abcd");
    expect(g().past.length).toBe(steps + 2);
    g().undo();
    g().undo();
    expect(g().nodes[0].data.params.key).toBe("");
  });

  it("does not resurrect old results on undo", () => {
    const id = g().addNode(desc("a"), { x: 0, y: 0 });
    g().updateRuntime(id, { status: "done", outputs: { text: { type: "text", value: "old" } } });
    vi.advanceTimersByTime(2000);
    g().setParam(id, "key", "k");
    g().updateRuntime(id, { status: "done", outputs: { text: { type: "text", value: "new" } } });
    g().undo();
    expect(g().nodes[0].data.params.key).toBe("");
    expect(g().nodes[0].data.outputs?.text).toEqual({ type: "text", value: "new" });
  });

  it("collapses a transaction into one step and drops empty ones", () => {
    const steps = g().past.length;
    g().transact(() => {
      g().addNode(desc("a"), { x: 0, y: 0 });
      g().addNode(desc("b"), { x: 100, y: 0 });
    });
    expect(g().past.length).toBe(steps + 1);
    g().transact(() => {});
    expect(g().past.length).toBe(steps + 1);
    g().undo();
    expect(g().nodes).toHaveLength(0);
  });

  it("returns to the saved revision when undoing back to it", () => {
    const id = g().addNode(desc("a"), { x: 0, y: 0 });
    const saved = g().editRevision;
    vi.advanceTimersByTime(2000);
    g().setParam(id, "key", "x");
    expect(g().editRevision).not.toBe(saved);
    g().undo();
    expect(g().editRevision).toBe(saved);
    g().redo();
    expect(g().editRevision).not.toBe(saved);
  });
});

describe("graph edits", () => {
  it("selects only the newly added node", () => {
    const a = g().addNode(desc("a"), { x: 0, y: 0 });
    const b = g().addNode(desc("b"), { x: 100, y: 0 });
    expect(g().selectedId).toBe(b);
    expect(g().nodes.find((n) => n.id === a)?.selected).toBe(false);
    expect(g().nodes.find((n) => n.id === b)?.selected).toBe(true);
  });

  it("replaces an existing wire into the same input", () => {
    const a = g().addNode(desc("a"), { x: 0, y: 0 });
    const b = g().addNode(desc("b"), { x: 0, y: 100 });
    const c = g().addNode(desc("c"), { x: 200, y: 0 });
    g().onConnect({ source: a, sourceHandle: "text", target: c, targetHandle: "text" });
    g().onConnect({ source: b, sourceHandle: "text", target: c, targetHandle: "text" });
    expect(g().edges).toHaveLength(1);
    expect(g().edges[0].source).toBe(b);
  });

  it("deletes the whole selection as one step", () => {
    g().addNode(desc("a"), { x: 0, y: 0 });
    g().addNode(desc("b"), { x: 100, y: 0 });
    g().selectAll();
    const steps = g().past.length;
    expect(g().deleteSelection()).toBe(true);
    expect(g().nodes).toHaveLength(0);
    expect(g().past.length).toBe(steps + 1);
    expect(g().deleteSelection()).toBe(false);
  });

  it("keeps results of unchanged nodes on a recorded reload", () => {
    const id = g().addNode(desc("a"), { x: 0, y: 0 });
    g().updateRuntime(id, { status: "done", outputs: { text: { type: "text", value: "r" } } });
    const n = g().nodes[0];
    const saved = {
      id,
      descriptorId: "a",
      label: n.data.label,
      color: n.data.color,
      params: { ...n.data.params },
      inputParams: [],
      disabled: false,
      position: { x: 40, y: 0 },
    };
    const steps = g().past.length;
    g().loadFlow([saved], [], { history: "record", keepRuntime: true });
    expect(g().nodes[0].data.status).toBe("done");
    expect(g().past.length).toBe(steps + 1);
    g().loadFlow([{ ...saved, params: { key: "changed" } }], [], { history: "record", keepRuntime: true });
    expect(g().nodes[0].data.status).toBe("idle");
  });
});
