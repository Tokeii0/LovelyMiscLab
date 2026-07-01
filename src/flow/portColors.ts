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

/** Mirrors `PortType::accepts` in Rust: `any` matches anything, else exact. */
export function canConnect(source: PortType, target: PortType): boolean {
  return target === "any" || source === "any" || source === target;
}

/** The port type a param exposes when "converted to input" (driven by a node). */
export function paramPortType(widget: ParamWidget): PortType {
  switch (widget.kind) {
    case "number":
    case "slider":
      return "number";
    case "toggle":
      return "bool";
    default:
      return "text"; // text / select / file
  }
}
