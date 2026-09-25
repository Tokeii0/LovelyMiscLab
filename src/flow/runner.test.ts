import { beforeEach, describe, expect, it } from "vitest";

import { mockDescriptors } from "@/lib/devMocks";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";

import { requestRun, runGraph, stopRun } from "./runner";

const g = () => useGraphStore.getState();
const byId = (id: string) => mockDescriptors.find((d) => d.id === id)!;
const node = (id: string) => g().nodes.find((n) => n.id === id)!;

/** text_input → base64_decode → text_output */
function chain(text: string) {
  const a = g().addNode(byId("text_input"), { x: 0, y: 0 });
  g().setParam(a, "text", text);
  const b = g().addNode(byId("base64_decode"), { x: 200, y: 0 });
  const c = g().addNode(byId("text_output"), { x: 400, y: 0 });
  g().onConnect({ source: a, sourceHandle: "text", target: b, targetHandle: "text" });
  g().onConnect({ source: b, sourceHandle: "text", target: c, targetHandle: "text" });
  return { a, b, c };
}

beforeEach(() => {
  useDescriptorStore.getState().setDescriptors(mockDescriptors);
  g().loadFlow([], []);
  useRunStore.getState().setMode("manual");
});

describe("runner", () => {
  it("streams outputs onto nodes as they finish", async () => {
    const { b, c } = chain("ZmxhZw==");
    await runGraph();
    expect(node(b).data.status).toBe("done");
    expect(node(b).data.outputs?.text).toEqual({ type: "text", value: "flag" });
    expect(node(c).data.outputs?.value).toEqual({ type: "text", value: "flag" });
    expect(useRunStore.getState().running).toBe(false);
  });

  it("skips dependants of a failed node and clears its old output", async () => {
    const { a, b, c } = chain("ZmxhZw==");
    await runGraph();
    g().setParam(a, "text", "%%%not base64%%%");
    await runGraph();
    expect(node(b).data.status).toBe("error");
    expect(node(b).data.outputs).toBeUndefined();
    expect(node(c).data.status).toBe("skipped");
    expect(node(c).data.hint).toContain("跳过");
  });

  it("marks results downstream of an edit as stale until the next run", async () => {
    const { a, b, c } = chain("ZmxhZw==");
    await runGraph();
    g().setParam(a, "text", "b2s=");
    expect(node(b).data.stale).toBe(true);
    expect(node(c).data.stale).toBe(true);
    await runGraph();
    expect(node(c).data.stale).toBe(false);
    expect(node(c).data.outputs?.value).toEqual({ type: "text", value: "ok" });
  });

  it("stop keeps finished results and cancels the rest", async () => {
    const { a, c } = chain("ZmxhZw==");
    const run = runGraph();
    await new Promise((r) => setTimeout(r, 200)); // first node done, second running
    await stopRun();
    await run;
    expect(node(a).data.status).toBe("done");
    expect(node(a).data.outputs).toBeDefined();
    expect(node(c).data.status).not.toBe("running");
    expect(useRunStore.getState().history[0].status).toBe("cancelled");
  });

  it("coalesces requests made during a run into one follow-up run", async () => {
    chain("ZmxhZw==");
    const before = useRunStore.getState().history.length;
    const first = runGraph();
    void requestRun({ kind: "graph" });
    void requestRun({ kind: "graph" });
    void requestRun({ kind: "graph" });
    await first;
    await new Promise((r) => setTimeout(r, 800));
    expect(useRunStore.getState().history.length - before).toBe(2);
  });
});
