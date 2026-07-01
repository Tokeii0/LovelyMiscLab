import { useGraphStore } from "@/store/graph";

import type { NodeDescriptor, ParamSpec, PortSpec, PortType } from "./types";

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

// Base-N family (Base32/45/58/62/85/92) — generated as encode/decode pairs to
// keep the mock list readable. Mirrors the real backend descriptors.
const sel = (name: string, label: string, options: string[], def: string): ParamSpec => ({
  name,
  label,
  widget: { kind: "select", options },
  default: def,
});
const tog = (name: string, label: string, def: boolean): ParamSpec => ({
  name,
  label,
  widget: { kind: "toggle" },
  default: def,
});
const txt = (name: string, label: string, def: string): ParamSpec => ({
  name,
  label,
  widget: { kind: "text", multiline: false },
  default: def,
});

function basePair(
  id: string,
  name: string,
  encParams: ParamSpec[],
  decParams: ParamSpec[]
): void {
  mockDescriptors.push(
    {
      id: `${id}_encode`,
      category: "编码/加密",
      displayName: `${name} 编码`,
      color: "#3b82f6",
      inputs: [{ name: "data", label: "输入", type: "any", required: true }],
      outputs: [{ name: "text", label: "输出", type: "text", required: true }],
      params: encParams,
      cost: "cheap",
    },
    {
      id: `${id}_decode`,
      category: "编码/加密",
      displayName: `${name} 解码`,
      color: "#3b82f6",
      inputs: [{ name: "text", label: "输入", type: "text", required: true }],
      outputs: [
        { name: "text", label: "文本", type: "text", required: true },
        { name: "bytes", label: "字节", type: "bytes", required: false },
      ],
      params: decParams,
      cost: "cheap",
    }
  );
}

const b32v = () => sel("variant", "码表", ["标准", "Hex 扩展"], "标准");
const b58v = () => sel("variant", "码表", ["Bitcoin", "Ripple", "自定义"], "Bitcoin");
const b85v = () => sel("variant", "码表", ["标准", "Z85", "IPv6"], "标准");
const strip = () => tog("strip", "去除非码表字符", true);

basePair("base32", "Base32", [b32v()], [b32v(), strip()]);
basePair("base45", "Base45", [], [strip()]);
basePair(
  "base58",
  "Base58",
  [b58v(), txt("alphabet", "自定义码表(58字符)", "")],
  [b58v(), txt("alphabet", "自定义码表(58字符)", ""), strip()]
);
basePair("base62", "Base62", [txt("alphabet", "码表", "0-9A-Za-z")], [txt("alphabet", "码表", "0-9A-Za-z")]);
basePair("base85", "Base85", [b85v(), tog("delim", "包含 <~ ~> 分隔符", false)], [b85v(), strip()]);
basePair("base92", "Base92", [], []);

// Hash / radix / charset / cipher families.
const num = (name: string, label: string, min: number, max: number, step: number, def: number): ParamSpec => ({
  name,
  label,
  widget: { kind: "number", min, max, step },
  default: def,
});
const p = (name: string, label: string, type: PortType, required = true): PortSpec => ({
  name,
  label,
  type,
  required,
});
const anyIn = () => [p("data", "输入", "any")];
const textIn = () => [p("text", "输入", "text")];
const textOut = () => [p("text", "输出", "text")];
const decOut = () => [p("text", "文本", "text"), p("bytes", "字节", "bytes", false)];

function pushDesc(
  id: string,
  category: string,
  name: string,
  color: string,
  inputs: PortSpec[],
  outputs: PortSpec[],
  params: ParamSpec[]
): void {
  mockDescriptors.push({ id, category, displayName: name, color, inputs, outputs, params, cost: "cheap" });
}

const CHARSETS = [
  "UTF-8", "UTF-16LE", "UTF-16BE", "GBK", "GB18030", "Big5", "Shift-JIS",
  "EUC-JP", "EUC-KR", "Windows-1252", "Windows-1251", "ISO-8859-1", "KOI8-R",
];
const HASH_ALGOS = [
  "MD5", "MD4", "SHA1", "SHA224", "SHA256", "SHA384", "SHA512", "SHA3-256",
  "SHA3-512", "Keccak-256", "RIPEMD-160", "CRC32",
];
const CYAN = "#06b6d4", SLATE = "#64748b", ROSE = "#f43f5e", TEAL = "#14b8a6";
const fmt = (name: string, label: string, def: string) =>
  sel(name, label, ["UTF8", "Hex", "Base64"], def);

pushDesc("hash", "哈希/摘要", "哈希计算", CYAN, anyIn(), [p("text", "摘要(hex)", "text")], [sel("algorithm", "算法", HASH_ALGOS, "SHA256")]);
pushDesc("hmac", "哈希/摘要", "HMAC", CYAN, anyIn(), [p("text", "摘要(hex)", "text")], [sel("algorithm", "算法", ["SHA256", "SHA1", "MD5", "SHA512"], "SHA256"), txt("key", "密钥", ""), sel("keyFormat", "密钥格式", ["UTF8", "Hex", "Base64"], "UTF8")]);
pushDesc("radix_convert", "进制转换", "进制转换", SLATE, [p("text", "数字", "text")], [p("text", "结果", "text")], [num("from", "源进制", 2, 36, 1, 10), num("to", "目标进制", 2, 36, 1, 16)]);
pushDesc("to_binary", "进制转换", "转二进制", SLATE, anyIn(), textOut(), [sel("delimiter", "分隔符", ["空格", "无", "逗号"], "空格")]);
pushDesc("from_binary", "进制转换", "二进制转文本", SLATE, textIn(), decOut(), []);
pushDesc("to_decimal", "进制转换", "转十进制", SLATE, anyIn(), textOut(), [sel("delimiter", "分隔符", ["空格", "逗号"], "空格")]);
pushDesc("from_decimal", "进制转换", "十进制转文本", SLATE, textIn(), decOut(), []);
pushDesc("encode_text", "字符编码", "文本编码", TEAL, [p("text", "文本", "text")], [p("hex", "hex", "text"), p("bytes", "字节", "bytes", false)], [sel("charset", "字符集", CHARSETS, "UTF-8")]);
pushDesc("decode_text", "字符编码", "文本解码", TEAL, [p("data", "字节/文本", "any")], [p("text", "文本", "text")], [sel("charset", "字符集", CHARSETS, "UTF-8")]);
pushDesc("aes", "加密解密", "AES", ROSE, textIn(), decOut(), [sel("operation", "操作", ["加密", "解密"], "加密"), sel("mode", "模式", ["CBC", "ECB", "CTR"], "CBC"), txt("key", "密钥", ""), fmt("keyFormat", "密钥格式", "Hex"), txt("iv", "IV", ""), fmt("ivFormat", "IV 格式", "Hex"), fmt("inputFormat", "输入格式", "UTF8"), sel("outputFormat", "输出格式", ["Hex", "Base64", "UTF8"], "Hex")]);
pushDesc("rc4", "加密解密", "RC4", ROSE, textIn(), decOut(), [txt("key", "密钥", ""), fmt("keyFormat", "密钥格式", "UTF8"), fmt("inputFormat", "输入格式", "UTF8"), sel("outputFormat", "输出格式", ["Hex", "UTF8", "Base64"], "Hex")]);
pushDesc("vigenere", "加密解密", "维吉尼亚密码", ROSE, textIn(), textOut(), [sel("operation", "操作", ["加密", "解密"], "加密"), txt("key", "密钥(字母)", "KEY")]);
pushDesc("affine", "加密解密", "仿射密码", ROSE, textIn(), textOut(), [sel("operation", "操作", ["加密", "解密"], "加密"), num("a", "a (与26互质)", 1, 25, 1, 5), num("b", "b", 0, 25, 1, 8)]);
pushDesc("atbash", "加密解密", "Atbash", ROSE, textIn(), textOut(), []);
pushDesc("rot47", "加密解密", "ROT47", ROSE, textIn(), textOut(), []);

// Control / logic
const AMBER = "#f59e0b";
const XFORMS = ["大写", "小写", "反转", "去空白", "Base64编码", "Base64解码", "Hex编码", "Hex解码", "URL编码", "URL解码", "ROT13", "MD5", "SHA1", "SHA256"];
pushDesc("switch", "控制/逻辑", "条件选择", AMBER, [p("condition", "条件", "bool"), p("a", "真", "any"), p("b", "假", "any")], [p("output", "输出", "any")], []);
pushDesc("logic", "控制/逻辑", "逻辑运算", AMBER, [p("a", "A", "bool"), p("b", "B", "bool", false)], [p("result", "结果", "bool")], [sel("op", "运算", ["AND", "OR", "NOT", "XOR", "NAND", "NOR"], "AND")]);
pushDesc("switch_case", "控制/逻辑", "多路分支", AMBER, [p("selector", "选择器", "any"), p("case0", "分支0", "any", false), p("case1", "分支1", "any", false), p("case2", "分支2", "any", false), p("case3", "分支3", "any", false), p("default", "默认", "any", false)], [p("output", "输出", "any")], []);
pushDesc("gate", "控制/逻辑", "条件门", AMBER, [p("value", "值", "any"), p("condition", "条件", "bool")], [p("output", "输出", "any"), p("passed", "已通过", "bool", false)], []);
pushDesc("range", "控制/逻辑", "数值范围", AMBER, [], [p("list", "序列", "stringList"), p("count", "数量", "number", false)], [num("start", "起始", -1000000, 1000000, 1, 0), num("end", "结束(不含)", -1000000, 1000000, 1, 10), num("step", "步长", -1000000, 1000000, 1, 1)]);
pushDesc("map", "控制/逻辑", "逐项映射", AMBER, [p("list", "列表", "stringList")], [p("list", "结果", "stringList")], [sel("op", "操作", XFORMS, "大写")]);
pushDesc("filter_list", "控制/逻辑", "列表过滤", AMBER, [p("list", "列表", "stringList")], [p("list", "结果", "stringList"), p("count", "数量", "number", false)], [txt("pattern", "正则", "."), sel("mode", "模式", ["保留匹配", "排除匹配"], "保留匹配")]);
pushDesc("join_list", "控制/逻辑", "列表合并", AMBER, [p("list", "列表", "stringList")], [p("text", "文本", "text")], [sel("sep", "分隔符", ["换行", "逗号", "空格", "无"], "换行")]);
pushDesc("iterate", "控制/逻辑", "迭代循环", AMBER, textIn(), [p("text", "结果", "text"), p("iterations", "迭代次数", "number", false), p("hit", "命中", "bool", false)], [sel("op", "操作", XFORMS, "Base64解码"), txt("until", "停止正则(可选)", "flag\\{[^}]*\\}"), num("max", "最大次数", 1, 100, 1, 16)]);

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
