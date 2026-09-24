import { describe, expect, it } from "vitest";

import { choose, confirmDialog, promptDialog, useConfirmStore } from "@/store/confirm";

describe("confirm queue", () => {
  it("resolves requests one at a time, in order", async () => {
    const a = confirmDialog({ title: "A" });
    const b = promptDialog({ title: "B", initial: "x" });
    const c = choose({ title: "C", options: [{ value: "save", label: "保存" }] });
    expect(useConfirmStore.getState().queue.map((r) => r.title)).toEqual(["A", "B", "C"]);

    useConfirmStore.getState().settle(true);
    useConfirmStore.getState().settle(null);
    useConfirmStore.getState().settle("save");

    await expect(a).resolves.toBe(true);
    await expect(b).resolves.toBeNull();
    await expect(c).resolves.toBe("save");
    expect(useConfirmStore.getState().queue).toHaveLength(0);
  });
});
