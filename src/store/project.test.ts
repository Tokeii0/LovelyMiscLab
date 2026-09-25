import { beforeEach, describe, expect, it, vi } from "vitest";

import type { NodeDescriptor } from "@/lib/types";
import { useGraphStore } from "@/store/graph";
import { isProjectDirty, useProjectStore } from "@/store/project";

const desc: NodeDescriptor = {
  id: "a",
  category: "测试",
  displayName: "A",
  color: "#888",
  inputs: [],
  outputs: [],
  params: [{ name: "k", label: "k", widget: { kind: "text", multiline: false }, default: "" }],
  cost: "cheap",
};

const g = () => useGraphStore.getState();
const p = () => useProjectStore.getState();

beforeEach(() => {
  vi.useFakeTimers();
  g().loadFlow([], []);
  p().reset();
  p().markSaved(g().editRevision);
  vi.advanceTimersByTime(5000);
});

describe("dirty tracking", () => {
  it("an empty, never-saved canvas has nothing to lose", () => {
    p().markDirty();
    expect(isProjectDirty()).toBe(false);
  });

  it("edits make the project dirty and saving clears it", () => {
    g().addNode(desc, { x: 0, y: 0 });
    expect(isProjectDirty()).toBe(true);
    p().markSaved(g().editRevision);
    expect(isProjectDirty()).toBe(false);
  });

  it("undoing back to the saved state is clean again", () => {
    const id = g().addNode(desc, { x: 0, y: 0 });
    p().setPath("/tmp/x.lml");
    p().markSaved(g().editRevision);
    vi.advanceTimersByTime(2000);
    g().setParam(id, "k", "v");
    expect(isProjectDirty()).toBe(true);
    g().undo();
    expect(isProjectDirty()).toBe(false);
  });

  it("selection and run results are not edits", () => {
    const id = g().addNode(desc, { x: 0, y: 0 });
    p().markSaved(g().editRevision);
    g().deselectAll();
    g().updateRuntime(id, { status: "done" });
    expect(isProjectDirty()).toBe(false);
  });

  it("detach forgets the file and counts as unsaved", () => {
    g().addNode(desc, { x: 0, y: 0 });
    p().setPath("/tmp/old.lml");
    p().markSaved(g().editRevision);
    p().detach("模板");
    expect(p().path).toBeNull();
    expect(p().name).toBe("模板");
    expect(isProjectDirty()).toBe(true);
  });
});
