// Canvas-level actions shared by the title bar, context menu and command palette.

import { confirmDialog } from "@/store/confirm";
import { useGraphStore } from "@/store/graph";

/** Remove every node and edge after confirming. Undoable. */
export async function clearCanvas() {
  if (useGraphStore.getState().nodes.length === 0) return;
  const ok = await confirmDialog({
    title: "清空画布？",
    message: "将删除画布上的全部节点与连线（可用 Ctrl+Z 撤销）。",
    confirmText: "清空",
    danger: true,
  });
  if (ok) useGraphStore.getState().clear();
}
