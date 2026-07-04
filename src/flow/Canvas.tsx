import { useCallback, useEffect, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  useReactFlow,
} from "@xyflow/react";

import type { NodeDescriptor } from "@/lib/types";
import { AgentPanel } from "@/app/AgentPanel";
import { useAgentStore } from "@/store/agent";
import { useGraphStore } from "@/store/graph";
import { usePaletteDrag } from "@/store/paletteDrag";
import { usePortSuggest } from "@/store/portSuggest";
import { useThemeStore } from "@/store/theme";

import { runAgent } from "./agentRunner";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { viewportAspect } from "./layout";
import { GenericNode } from "./GenericNode";
import { LabeledEdge } from "./LabeledEdge";
import { NodeSearchMenu } from "./NodeSearchMenu";
import { canConnect } from "./portColors";
import { PortSuggest } from "./PortSuggest";
import { resolvePortType } from "./portUtils";
import { executeGraph } from "./runner";
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

type Search = { x: number; y: number; flow: { x: number; y: number } };

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

  // Resolve a palette drop: add at the cursor if over the canvas, else (a plain
  // click) add near the canvas center.
  useEffect(() => {
    setDrop((d, x, y, moved) => {
      const el = document.elementFromPoint(x, y);
      const overCanvas = !!(el && el.closest(".react-flow"));
      if (overCanvas) {
        addNode(d, rf.screenToFlowPosition({ x, y }));
      } else if (!moved) {
        addNode(
          d,
          rf.screenToFlowPosition({
            x: window.innerWidth / 2 - 90,
            y: window.innerHeight / 2 - 40,
          })
        );
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
      if (!s || !t) return false;
      return canConnect(s, t);
    },
    []
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
    if (search) addNode(d, search.flow);
    setSearch(null);
  };

  const menuItems = (): MenuItem[] => {
    if (!menu) return [];
    const g = useGraphStore.getState();
    if (menu.kind === "node") {
      const id = menu.id!;
      return [
        { label: "复制节点", onClick: () => g.duplicateNode(id) },
        { label: "删除节点", danger: true, onClick: () => g.deleteNode(id) },
      ];
    }
    if (menu.kind === "edge") {
      const id = menu.id!;
      return [{ label: "删除连线", danger: true, onClick: () => g.deleteEdge(id) }];
    }
    const { x, y, flow } = menu;
    return [
      { label: "添加节点…", onClick: () => flow && setSearch({ x, y, flow }) },
      { label: "运行整图", onClick: () => void executeGraph() },
      {
        label: "整理节点",
        onClick: () => {
          g.arrangeNodes(viewportAspect());
          // Let React Flow apply the new positions before fitView measures them.
          requestAnimationFrame(() => rf.fitView({ duration: 250, padding: 0.15 }));
        },
      },
      { label: "适应视图", onClick: () => rf.fitView({ duration: 200 }) },
      { label: "全选节点", onClick: () => g.selectAll() },
      { label: "清空画布", danger: true, onClick: () => g.clear() },
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

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} items={menuItems()} onClose={closeMenu} />
      )}
      {search && (
        <NodeSearchMenu
          x={search.x}
          y={search.y}
          onPick={onPick}
          onClose={() => setSearch(null)}
        />
      )}
      {suggestCtx && <PortSuggest />}
      <AgentPanel />
    </div>
  );
}
