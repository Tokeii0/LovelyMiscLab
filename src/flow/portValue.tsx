import type { PortValue } from "@/lib/types";
import { useImageViewer } from "@/store/imageViewer";

/** A PortValue rendered as plain text (for copy / non-visual types). */
export function valueText(v: PortValue): string {
  switch (v.type) {
    case "text":
      return v.value;
    case "number":
      return String(v.value);
    case "bool":
      return v.value ? "true" : "false";
    case "stringList":
      return v.value.join("\n");
    case "candidates":
      return v.value.map((c) => `${c.score.toFixed(2)}  ${c.text}`).join("\n");
    case "bytes":
      return `<${v.value.length} 字节>`;
    case "artifact":
    case "image":
      return v.value;
    case "json":
    case "fingerprint":
      return JSON.stringify(v.value, null, 2);
    default:
      return "";
  }
}

/** First `limit` bytes as spaced hex (with an ellipsis when truncated). */
export function bytesToHex(bytes: number[], limit = 512): string {
  const shown = bytes
    .slice(0, limit)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join(" ");
  return bytes.length > limit ? `${shown} …` : shown;
}

/**
 * Render a port value by its actual type: image → picture, bytes → hex dump,
 * everything else → its text form. Shared by the node inspector and run history
 * so results look identical everywhere.
 */
export function OutputValue({ value }: { value: PortValue }) {
  if (value.type === "none") {
    return <span className="text-[11px] text-muted-foreground">（无输出）</span>;
  }
  if (value.type === "image") {
    return (
      <img
        src={value.value}
        alt="output"
        title="点击查看大图 / 调整"
        onClick={() => useImageViewer.getState().show(value.value)}
        className="max-h-56 max-w-full cursor-zoom-in rounded border border-border bg-white object-contain"
      />
    );
  }
  if (value.type === "bytes") {
    return (
      <div>
        <div className="mb-1 text-[10px] text-muted-foreground">{value.value.length} 字节</div>
        <pre className="max-h-44 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-background p-2 font-mono text-[10px] leading-snug">
          {value.value.length ? bytesToHex(value.value) : "（空）"}
        </pre>
      </div>
    );
  }
  return (
    <pre className="max-h-56 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-background p-2 font-mono text-[10px] leading-snug">
      {valueText(value) || "（空）"}
    </pre>
  );
}
