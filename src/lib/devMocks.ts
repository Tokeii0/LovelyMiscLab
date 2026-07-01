import { useGraphStore } from "@/store/graph";

import type { NodeDescriptor } from "./types";

/** True when running inside the Tauri webview (IPC available). */
export const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Fallback descriptors so the canvas is populated when running in a plain
// browser (e.g. `pnpm dev` preview) where Tauri IPC is unavailable. These are
// NEVER used inside the app — there `list_node_descriptors` returns the real set.
export const mockDescriptors: NodeDescriptor[] = [
  {
    id: "text_input",
    category: "输入输出",
    displayName: "文本输入",
    color: "#64748b",
    inputs: [],
    outputs: [{ name: "text", label: "文本", type: "text", required: true }],
    params: [
      {
        name: "text",
        label: "文本",
        widget: { kind: "text", multiline: true },
        default: "ZmxhZ3ttaXNjX2Zsb3dfaXNfZnVufQ==",
      },
    ],
    cost: "cheap",
  },
  {
    id: "file_import",
    category: "输入输出",
    displayName: "文件导入",
    color: "#64748b",
    inputs: [],
    outputs: [{ name: "bytes", label: "字节", type: "bytes", required: true }],
    params: [{ name: "path", label: "文件", widget: { kind: "file" }, default: "" }],
    cost: "cheap",
  },
  {
    id: "base64_decode",
    category: "编码/加密",
    displayName: "Base64 解码",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [{ name: "variant", label: "码表", widget: { kind: "select", options: ["标准", "URL安全"] }, default: "标准" }],
    cost: "cheap",
  },
  {
    id: "base64_encode",
    category: "编码/加密",
    displayName: "Base64 编码",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [],
    cost: "cheap",
  },
  {
    id: "qr_encode",
    category: "编码/加密",
    displayName: "二维码编码",
    color: "#14b8a6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "image", label: "二维码", type: "image", required: true }],
    params: [{ name: "scale", label: "像素倍率", widget: { kind: "number", min: 1, max: 32, step: 1 }, default: 8 }],
    cost: "cheap",
  },
  {
    id: "text_output",
    category: "输入输出",
    displayName: "文本输出",
    color: "#22c55e",
    inputs: [{ name: "text", label: "文本", type: "text", required: true }],
    outputs: [],
    params: [],
    cost: "cheap",
  },
  {
    id: "hex_decode",
    category: "编码/加密",
    displayName: "Hex 解码",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [],
    cost: "cheap",
  },
  {
    id: "rot13",
    category: "编码/加密",
    displayName: "ROT13",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [],
    cost: "cheap",
  },
  {
    id: "magic_decode",
    category: "编码/加密",
    displayName: "魔法解码",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [
      { name: "text", label: "结果", type: "text", required: true },
      { name: "chain", label: "解码链", type: "text", required: false },
      { name: "hit", label: "命中", type: "bool", required: false },
    ],
    params: [
      { name: "pattern", label: "目标正则", widget: { kind: "text", multiline: false }, default: "flag\\{[^}]*\\}" },
      { name: "depth", label: "最大深度", widget: { kind: "number", min: 1, max: 16, step: 1 }, default: 8 },
    ],
    cost: "cheap",
  },
  {
    id: "loop_decode",
    category: "编码/加密",
    displayName: "循环解码",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [
      { name: "text", label: "结果", type: "text", required: true },
      { name: "iterations", label: "次数", type: "number", required: false },
      { name: "hit", label: "命中", type: "bool", required: false },
    ],
    params: [
      { name: "codec", label: "编码", widget: { kind: "select", options: ["Base64", "Hex", "URL"] }, default: "Base64" },
      { name: "until", label: "退出条件", widget: { kind: "select", options: ["无法继续", "匹配正则"] }, default: "无法继续" },
      { name: "pattern", label: "正则", widget: { kind: "text", multiline: false }, default: "flag\\{[^}]*\\}" },
      { name: "max", label: "最大次数", widget: { kind: "number", min: 1, max: 100, step: 1 }, default: 16 },
    ],
    cost: "cheap",
  },
  {
    id: "xor_bruteforce",
    category: "编码/加密",
    displayName: "XOR 爆破",
    color: "#3b82f6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [
      { name: "best", label: "最佳", type: "text", required: true },
      { name: "candidates", label: "候选", type: "candidates", required: false },
    ],
    params: [],
    cost: "medium",
  },
  {
    id: "regex_extract",
    category: "文本处理",
    displayName: "正则提取",
    color: "#14b8a6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [
      { name: "text", label: "首个匹配", type: "text", required: true },
      { name: "matches", label: "全部匹配", type: "stringList", required: false },
    ],
    params: [
      {
        name: "preset",
        label: "预设",
        widget: { kind: "select", options: ["自定义", "flag", "MD5", "SHA1", "IPv4", "邮箱", "URL", "Base64块", "Hex串"] },
        default: "flag",
      },
      { name: "pattern", label: "自定义正则", widget: { kind: "text", multiline: false }, default: "flag\\{[^}]*\\}" },
    ],
    cost: "cheap",
  },
  {
    id: "qr_decode",
    category: "编码/加密",
    displayName: "二维码解码",
    color: "#14b8a6",
    inputs: [{ name: "image", label: "图片字节", type: "bytes", required: true }],
    outputs: [
      { name: "text", label: "内容", type: "text", required: true },
      { name: "all", label: "全部", type: "stringList", required: false },
      { name: "format", label: "格式", type: "text", required: false },
    ],
    params: [],
    cost: "medium",
  },
  {
    id: "archive_extract",
    category: "压缩包",
    displayName: "解压缩",
    color: "#f59e0b",
    inputs: [{ name: "archive", label: "压缩包字节", type: "bytes", required: true }],
    outputs: [
      { name: "files", label: "文件列表", type: "stringList", required: true },
      { name: "text", label: "内容", type: "text", required: false },
      { name: "bytes", label: "字节", type: "bytes", required: false },
    ],
    params: [
      { name: "format", label: "格式", widget: { kind: "select", options: ["自动", "zip", "7z", "rar", "gz", "tar"] }, default: "自动" },
      { name: "password", label: "密码", widget: { kind: "text", multiline: false }, default: "" },
      { name: "entry", label: "指定条目(可选)", widget: { kind: "text", multiline: false }, default: "" },
    ],
    cost: "medium",
  },
  {
    id: "reverse",
    category: "文本处理",
    displayName: "文本反转",
    color: "#14b8a6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [],
    cost: "cheap",
  },
  {
    id: "concat",
    category: "文本处理",
    displayName: "文本拼接",
    color: "#14b8a6",
    inputs: [
      { name: "a", label: "A", type: "text", required: true },
      { name: "b", label: "B", type: "text", required: true },
    ],
    outputs: [{ name: "text", label: "输出", type: "text", required: true }],
    params: [{ name: "sep", label: "分隔符", widget: { kind: "text", multiline: false }, default: "" }],
    cost: "cheap",
  },
  {
    id: "text_score",
    category: "文本处理",
    displayName: "可读性评分",
    color: "#14b8a6",
    inputs: [{ name: "text", label: "输入", type: "text", required: true }],
    outputs: [
      { name: "score", label: "可读性", type: "number", required: true },
      { name: "flag", label: "疑似 flag", type: "text", required: false },
    ],
    params: [],
    cost: "cheap",
  },
  {
    id: "compare",
    category: "控制/逻辑",
    displayName: "比较",
    color: "#f59e0b",
    inputs: [
      { name: "a", label: "A", type: "text", required: true },
      { name: "b", label: "B", type: "text", required: true },
    ],
    outputs: [{ name: "result", label: "结果", type: "bool", required: true }],
    params: [
      {
        name: "op",
        label: "运算",
        widget: { kind: "select", options: ["==", "!=", "包含", "开头", "结尾", "匹配正则"] },
        default: "==",
      },
    ],
    cost: "cheap",
  },
  {
    id: "zero_width_decode",
    category: "隐写术",
    displayName: "零宽解码",
    color: "#6366f1",
    inputs: [{ name: "text", label: "载体文本", type: "text", required: true }],
    outputs: [
      { name: "text", label: "结果", type: "text", required: true },
      { name: "bits", label: "位串", type: "text", required: false },
      { name: "report", label: "分析", type: "text", required: false },
    ],
    params: [
      { name: "scheme", label: "模式", widget: { kind: "select", options: ["自动", "二进制"] }, default: "自动" },
      {
        name: "zero",
        label: "0 = 字符",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)"] },
        default: "ZWSP (U+200B)",
      },
      {
        name: "one",
        label: "1 = 字符",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)"] },
        default: "ZWNJ (U+200C)",
      },
      { name: "msb", label: "高位在前 (MSB)", widget: { kind: "toggle" }, default: true },
    ],
    cost: "cheap",
  },
  {
    id: "zero_width_encode",
    category: "隐写术",
    displayName: "零宽编码",
    color: "#6366f1",
    inputs: [{ name: "text", label: "秘密信息", type: "text", required: true }],
    outputs: [
      { name: "text", label: "结果", type: "text", required: true },
      { name: "bits", label: "位串", type: "text", required: false },
    ],
    params: [
      { name: "cover", label: "载体文本", widget: { kind: "text", multiline: false }, default: "The quick brown fox" },
      {
        name: "zero",
        label: "0 = 字符",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)"] },
        default: "ZWSP (U+200B)",
      },
      {
        name: "one",
        label: "1 = 字符",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)"] },
        default: "ZWNJ (U+200C)",
      },
      { name: "position", label: "隐藏位置", widget: { kind: "select", options: ["结尾", "开头", "中间"] }, default: "结尾" },
      { name: "msb", label: "高位在前 (MSB)", widget: { kind: "toggle" }, default: true },
    ],
    cost: "cheap",
  },
];

const DEMO_IMAGE =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Crect width='100%25' height='100%25' fill='%23000'/%3E%3Crect x='16' y='16' width='88' height='88' fill='%23fff'/%3E%3Ctext x='60' y='66' font-size='18' text-anchor='middle'%3EQR%3C/text%3E%3C/svg%3E";

/** Seed a demo graph (browser preview only) so the canvas isn't empty. */
export function seedDemo() {
  const g = useGraphStore.getState();
  if (g.nodes.length > 0) return;
  const byId = (id: string) => mockDescriptors.find((d) => d.id === id)!;
  const a = g.addNode(byId("text_input"), { x: 40, y: 80 });
  const b = g.addNode(byId("base64_decode"), { x: 300, y: 80 });
  const c = g.addNode(byId("text_output"), { x: 560, y: 80 });
  g.onConnect({ source: a, sourceHandle: "text", target: b, targetHandle: "text" });
  g.onConnect({ source: b, sourceHandle: "text", target: c, targetHandle: "text" });
  g.setSelected(b);
  g.updateRuntime(b, {
    status: "done",
    outputs: { text: { type: "text", value: "flag{misc_flow_is_fun}" } },
  });
  g.updateRuntime(c, {
    status: "done",
    outputs: { value: { type: "text", value: "flag{misc_flow_is_fun}" } },
  });
  const qr = g.addNode(byId("qr_encode"), { x: 300, y: 260 });
  g.updateRuntime(qr, {
    status: "done",
    outputs: { image: { type: "image", value: DEMO_IMAGE } },
  });
}
