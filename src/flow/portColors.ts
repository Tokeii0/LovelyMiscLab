import type { ParamWidget, PortType } from "@/lib/types";

/** Color per port type — shared visual language for handles + edges. */
export function portColor(t: PortType): string {
  switch (t) {
    case "text":
      return "#22c55e";
    case "number":
      return "#f59e0b";
    case "bool":
      return "#a855f7";
    case "json":
      return "#38bdf8";
    case "stringList":
      return "#14b8a6";
    case "candidates":
      return "#ec4899";
    case "bytes":
      return "#ef4444";
    case "artifact":
      return "#8b5cf6";
    case "image":
      return "#f472b6";
    case "fingerprint":
      return "#eab308";
    case "any":
    default:
      return "#94a3b8";
  }
}

const PORT_TYPE_LABEL: Record<PortType, string> = {
  any: "任意",
  text: "文本",
  number: "数值",
  bool: "布尔",
  json: "JSON",
  stringList: "文本列表",
  candidates: "候选列表",
  bytes: "字节",
  artifact: "文件",
  image: "图片",
  fingerprint: "指纹",
};

/** Human-readable (Chinese) name of a port type for tooltips, docs and labels. */
export function portTypeLabel(t: PortType): string {
  return PORT_TYPE_LABEL[t] ?? t;
}

/** Mirrors `PortType::accepts` in Rust: `any` matches anything, exact matches,
 * and a `text` input accepts scalar/list sources (coerced to string at the node
 * boundary) so e.g. a width/height number can drive a text field or 文本输出. */
export function canConnect(source: PortType, target: PortType): boolean {
  if (target === "any" || source === "any" || source === target) return true;
  return (
    target === "text" &&
    (source === "number" || source === "bool" || source === "stringList")
  );
}

/** The port type a param exposes when "converted to input" (driven by a node). */
export function paramPortType(widget: ParamWidget): PortType {
  switch (widget.kind) {
    case "number":
    case "slider":
      return "number";
    case "toggle":
      return "bool";
    case "image":
      return "image";
    default:
      return "text"; // text / select / file
  }
}
