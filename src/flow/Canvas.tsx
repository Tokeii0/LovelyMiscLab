import { useCallback, useEffect, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  useReactFlow,
  type FinalConnectionState,
} from "@xyflow/react";

import type { NodeDescriptor, PortType } from "@/lib/types";
import { AgentPanel } from "@/app/AgentPanel";
import { useAgentStore } from "@/store/agent";
import { useGraphStore } from "@/store/graph";
import { usePaletteDrag } from "@/store/paletteDrag";
import { usePortSuggest } from "@/store/portSuggest";
import { useThemeStore } from "@/store/theme";
import { clearCanvas } from "@/flow/canvasActions";
import { copySelection, duplicateSelection, hasClipboard, pasteClipboard } from "@/flow/clipboard";
import { placeInView, registerFlow } from "@/flow/placement";
import { promptDialog } from "@/store/confirm";
import { useHelpStore } from "@/store/help";
import { useModuleDialogStore } from "@/store/moduleDialog";

import { runAgent } from "./agentRunner";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { viewportAspect } from "./layout";
import { GenericNode } from "./GenericNode";
import { LabeledEdge } from "./LabeledEdge";
import { NodeSearchMenu } from "./NodeSearchMenu";
import { canConnect, portTypeLabel } from "./portColors";
import { PortSuggest } from "./PortSuggest";
import {
  candidateNodes,
  firstCompatibleInput,
  firstCompatibleOutput,
  resolvePortType,
} from "./portUtils";
import { runGraph, runNode, runToNode } from "./runner";
import { SelectorNode } from "./SelectorNode";

const nodeTypes = { generic: GenericNode, selector: SelectorNode };
const edgeTypes = { labeled: LabeledEdge };

type Menu = {
  x: number;
  y: number;
  kind: "pane" | "node" | "edge";
  id?: string;
  flow?: { x: number; y: number };
};

type Search = {
  x: number;
  y: number;
  flow: { x: number; y: number };
  /** Set when opened by dropping a wire: connect the picked node to this port. */
  wire?: { nodeId: string; port: string; dir: "in" | "out"; type: PortType };
};

/** Would a new edge source→target close a cycle (is source reachable from target)? */
function createsCycle(
  edges: { source: string; target: string }[],
  source: string,
  target: string
): boolean {
  const out = new Map<string, string[]>();
  for (const e of edges) out.set(e.source, [...(out.get(e.source) ?? []), e.target]);
  const stack = [target];
  const seen = new Set<string>();
  while (stack.length) {
    const n = stack.pop()!;
    if (n === source) return true;
    if (seen.has(n)) continue;
    seen.add(n);
    stack.push(...(out.get(n) ?? []));
  }
  return false;
}

export function Canvas() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const onNodesChange = useGraphStore((s) => s.onNodesChange);
  const onEdgesChange = useGraphStore((s) => s.onEdgesChange);
  const onConnect = useGraphStore((s) => s.onConnect);
  const addNode = useGraphStore((s) => s.addNode);
  const setSelected = useGraphStore((s) => s.setSelected);
  const setDrop = usePaletteDrag((s) => s.setDrop);
  const theme = useThemeStore((s) => s.theme);
  const rf = useReactFlow();
  const suggestCtx = usePortSuggest((s) => s.ctx);
  const pendingPrompt = useAgentStore((s) => s.pendingPrompt);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [search, setSearch] = useState<Search | null>(null);

  // The AI dialog hands the agent prompt off here (Canvas owns the ReactFlow
  // instance the agent needs for camera-follow). Consume it once.
  useEffect(() => {
    if (!pendingPrompt) return;
    const { pendingData } = useAgentStore.getState();
    useAgentStore.getState().clearPending();
    void runAgent(pendingPrompt, pendingData, rf);
  }, [pendingPrompt, rf]);

  useEffect(() => {
    registerFlow(rf);
    return () => registerFlow(null);
  }, [rf]);

  // Resolve a palette drop: add at the cursor if over the canvas, else (a plain
  // click) add at a free spot in the middle of the view.
  useEffect(() => {
    setDrop((d, x, y, moved) => {
      const el = document.elementFromPoint(x, y);
      const overCanvas = !!(el && el.closest(".react-flow"));
      if (overCanvas) {
        addNode(d, rf.screenToFlowPosition({ x, y }));
      } else if (!moved) {
        addNode(d, placeInView());
      }
    });
  }, [setDrop, addNode, rf]);

  const isValidConnection = useCallback(
    (c: {
      source?: string | null;
      target?: string | null;
      sourceHandle?: string | null;
      targetHandle?: string | null;
    }) => {
      if (!c.source || !c.target || !c.sourceHandle || !c.targetHandle) return false;
      if (c.source === c.target) return false;
      const s = resolvePortType(c.source, c.sourceHandle, "out");
      const t = resolvePortType(c.target, c.targetHandle, "in");
      if (!s || !t || !canConnect(s, t)) return false;
      // The engine runs a DAG: refuse a wire that would close a loop.
      return !createsCycle(useGraphStore.getState().edges, c.source, c.target);
    },
    []
  );

  // A wire dropped on empty canvas opens the search, pre-filtered to nodes that
  // can take it, and connects whatever is picked.
  const onConnectEnd = useCallback(
    (event: MouseEvent | TouchEvent, state: FinalConnectionState) => {
      if (state.isValid || !state.fromNode || !state.fromHandle?.id) return;
      const target = event.target as HTMLElement | null;
      if (!target?.classList.contains("react-flow__pane")) return;
      const point = "changedTouches" in event ? event.changedTouches[0] : event;
      const dir: "in" | "out" = state.fromHandle.type === "source" ? "out" : "in";
      const port = state.fromHandle.id;
      const type = resolvePortType(state.fromNode.id, port, dir);
      if (!type) return;
      setSearch({
        x: point.clientX,
        y: point.clientY,
        flow: rf.screenToFlowPosition({ x: point.clientX, y: point.clientY }),
        wire: { nodeId: state.fromNode.id, port, dir, type },
      });
    },
    [rf]
  );

  const closeMenu = useCallback(() => setMenu(null), []);

  // Double-click empty canvas → open the node search picker at the cursor.
  const onDoubleClick = (e: React.MouseEvent) => {
    const el = e.target as HTMLElement;
    if (!el.classList.contains("react-flow__pane")) return;
    setSearch({
      x: e.clientX,
      y: e.clientY,
      flow: rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }),
    });
  };

  const onPick = (d: NodeDescriptor) => {
    if (search) {
      const wire = search.wire;
      if (!wire) {
        addNode(d, search.flow);
      } else {
        // Add + connect as one undo step; an upstream node sits left of the drop point.
        const g = useGraphStore.getState();
        g.transact(() => {
          const pos =
            wire.dir === "in" ? { x: search.flow.x - 220, y: search.flow.y } : search.flow;
          const id = g.addNode(d, pos);
          if (wire.dir === "out") {
            const match = firstCompatibleInput(d, wire.type);
            if (match) {
              if (match.isParam) g.toggleParamInput(id, match.port);
              g.onConnect({
                source: wire.nodeId,
                sourceHandle: wire.port,
                target: id,
                targetHandle: match.port,
              });
            }
          } else {
            const out = firstCompatibleOutput(d, wire.type);
            if (out)
              g.onConnect({
                source: id,
                sourceHandle: out,
                target: wire.nodeId,
                targetHandle: wire.port,
              });
          }
        });
      }
    }
    setSearch(null);
  };

  const menuItems = (): MenuItem[] => {
    if (!menu) return [];
    const g = useGraphStore.getState();
    if (menu.kind === "node") {
      const id = menu.id!;
      const node = g.nodes.find((n) => n.id === id);
      const selected = g.nodes.filter((n) => n.selected);
      // Right-clicking inside a multi-selection acts on the whole selection.
      if (node?.selected && selected.length > 1) {
        return [
          {
            label: `复制 ${selected.length} 个节点`,
            hint: "Ctrl+C",
            onClick: () => void copySelection(),
          },
          { label: "创建副本", hint: "Ctrl+D", onClick: () => void duplicateSelection() },
          {
            label: "封装为模块…",
            onClick: () => useModuleDialogStore.getState().setOpen(true),
          },
          {
            label: `删除 ${selected.length} 个节点`,
            hint: "Delete",
            danger: true,
            separator: true,
            onClick: () => void g.deleteSelection(),
          },
        ];
      }
      const disabled = node?.data.disabled ?? false;
      return [
        { label: "运行到此节点", onClick: () => void runToNode(id) },
        { label: "仅运行此节点", onClick: () => void runNode(id) },
        {
          label: "重命名…",
          separator: true,
          onClick: async () => {
            const next = await promptDialog({
              title: "重命名节点",
              initial: node?.data.label ?? "",
            });
            if (next != null) g.renameNode(id, next.trim() || (node?.data.label ?? ""));
          },
        },
        { label: disabled ? "启用节点" : "禁用节点", onClick: () => g.setDisabled(id, !disabled) },
        { label: "创建副本", hint: "Ctrl+D", onClick: () => g.duplicateNode(id) },
        {
          label: "节点帮助",
          onClick: () => useHelpStore.getState().openForNode(node?.data.descriptorId),
        },
        {
          label: "删除节点",
          hint: "Delete",
          danger: true,
          separator: true,
          onClick: () => g.deleteNode(id),
        },
      ];
    }
    if (menu.kind === "edge") {
      const id = menu.id!;
      return [{ label: "删除连线", danger: true, onClick: () => g.deleteEdge(id) }];
    }
    const { x, y, flow } = menu;
    return [
      { label: "添加节点…", hint: "双击", onClick: () => flow && setSearch({ x, y, flow }) },
      {
        label: "粘贴",
        hint: "Ctrl+V",
        disabled: !hasClipboard(),
        onClick: () => void pasteClipboard(flow),
      },
      { label: "运行整图", hint: "Ctrl+Enter", separator: true, onClick: () => void runGraph() },
      {
        label: "整理节点（按数据流 左→右）",
        onClick: () => {
          g.arrangeNodes(viewportAspect(), "flow");
          requestAnimationFrame(() => rf.fitView({ duration: 250, padding: 0.15 }));
        },
      },
      {
        label: "整理节点（紧凑，适配视口）",
        onClick: () => {
          g.arrangeNodes(viewportAspect());
          // Let React Flow apply the new positions before fitView measures them.
          requestAnimationFrame(() => rf.fitView({ duration: 250, padding: 0.15 }));
        },
      },
      { label: "适应视图", onClick: () => rf.fitView({ duration: 200 }) },
      { label: "全选节点", hint: "Ctrl+A", onClick: () => g.selectAll() },
      { label: "清空画布…", danger: true, separator: true, onClick: () => void clearCanvas() },
    ];
  };

  return (
    <div className="relative h-full w-full" onDoubleClick={onDoubleClick}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={{
          type: "labeled",
          markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        }}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onConnectEnd={onConnectEnd}
        isValidConnection={isValidConnection as never}
        onSelectionChange={({ nodes }) => setSelected(nodes[0]?.id ?? null)}
        onPaneClick={closeMenu}
        onMoveStart={closeMenu}
        onPaneContextMenu={(e) => {
          e.preventDefault();
          setMenu({
            x: e.clientX,
            y: e.clientY,
            kind: "pane",
            flow: rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }),
          });
        }}
        onNodeContextMenu={(e, node) => {
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, kind: "node", id: node.id });
        }}
        onEdgeContextMenu={(e, edge) => {
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, kind: "edge", id: edge.id });
        }}
        onlyRenderVisibleElements
        deleteKeyCode={null}
        fitView
        zoomOnDoubleClick={false}
        colorMode={theme}
        proOptions={{ hideAttribution: true }}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={16}
          size={1}
          color={theme === "dark" ? "#2a3340" : "#cbd5e1"}
        />
        <Controls />
        <MiniMap
          pannable
          zoomable
          nodeColor={(n) => (n.data?.color as string) ?? "#64748b"}
          maskColor={theme === "dark" ? "#0d101799" : "#f1f5f999"}
          style={{ background: theme === "dark" ? "#151a22" : "#e2e8f0" }}
        />
      </ReactFlow>

      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems()} onClose={closeMenu} />}
      {search && (
        <NodeSearchMenu
          x={search.x}
          y={search.y}
          onPick={onPick}
          onClose={() => setSearch(null)}
          candidates={search.wire ? candidateNodes(search.wire.type, search.wire.dir) : undefined}
          title={
            search.wire
              ? `${search.wire.dir === "out" ? "接下游" : "接上游"}：${portTypeLabel(search.wire.type)}`
              : undefined
          }
        />
      )}
      {suggestCtx && <PortSuggest />}
      <AgentPanel />
    </div>
  );
}
