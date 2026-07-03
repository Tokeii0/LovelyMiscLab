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
    id: "image_input",
    category: "输入输出",
    displayName: "图片输入",
    color: "#64748b",
    inputs: [],
    outputs: [
      { name: "bytes", label: "字节", type: "bytes", required: true },
      { name: "image", label: "图片", type: "image", required: false },
      { name: "dataUrl", label: "数据URL", type: "text", required: false },
    ],
    params: [{ name: "image", label: "图片", widget: { kind: "image" }, default: "" }],
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
    outputs: [
      { name: "image", label: "二维码", type: "image", required: true },
      { name: "bytes", label: "PNG字节", type: "bytes", required: false },
    ],
    params: [
      { name: "ec", label: "纠错等级", widget: { kind: "select", options: ["L", "M", "Q", "H"] }, default: "M" },
      { name: "version", label: "版本(0=自动)", widget: { kind: "number", min: 0, max: 40, step: 1 }, default: 0 },
      { name: "scale", label: "像素倍率", widget: { kind: "number", min: 1, max: 64, step: 1 }, default: 8 },
      { name: "margin", label: "静默区(模块)", widget: { kind: "number", min: 0, max: 64, step: 1 }, default: 4 },
      { name: "dark", label: "前景色", widget: { kind: "text", multiline: false }, default: "#000000" },
      { name: "light", label: "背景色", widget: { kind: "text", multiline: false }, default: "#ffffff" },
    ],
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
    displayName: "解压",
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
      { name: "scheme", label: "模式", widget: { kind: "select", options: ["自动", "二进制", "四进制", "变体选择符", "Unicode标签"] }, default: "自动" },
      {
        name: "zero",
        label: "0 = 字符 (二进制)",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)", "LRM (U+200E)", "RLM (U+200F)", "INVISIBLE-TIMES (U+2062)", "ALM (U+061C)", "MVS (U+180E)"] },
        default: "ZWSP (U+200B)",
      },
      {
        name: "one",
        label: "1 = 字符 (二进制)",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)", "LRM (U+200E)", "RLM (U+200F)", "INVISIBLE-TIMES (U+2062)", "ALM (U+061C)", "MVS (U+180E)"] },
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
      { name: "scheme", label: "方案", widget: { kind: "select", options: ["二进制", "四进制", "变体选择符", "Unicode标签"] }, default: "二进制" },
      {
        name: "zero",
        label: "0 = 字符 (二进制)",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)", "LRM (U+200E)", "RLM (U+200F)", "INVISIBLE-TIMES (U+2062)", "ALM (U+061C)", "MVS (U+180E)"] },
        default: "ZWSP (U+200B)",
      },
      {
        name: "one",
        label: "1 = 字符 (二进制)",
        widget: { kind: "select", options: ["ZWSP (U+200B)", "ZWNJ (U+200C)", "ZWJ (U+200D)", "ZWNBSP (U+FEFF)", "WJ (U+2060)", "LRM (U+200E)", "RLM (U+200F)", "INVISIBLE-TIMES (U+2062)", "ALM (U+061C)", "MVS (U+180E)"] },
        default: "ZWNJ (U+200C)",
      },
      { name: "position", label: "隐藏位置", widget: { kind: "select", options: ["结尾", "开头", "中间"] }, default: "结尾" },
      { name: "msb", label: "高位在前 (MSB)", widget: { kind: "toggle" }, default: true },
    ],
    cost: "cheap",
  },
  {
    id: "stegcloak_hide",
    category: "隐写术",
    displayName: "StegCloak 编码",
    color: "#d946ef",
    inputs: [{ name: "text", label: "秘密信息", type: "text", required: true }],
    outputs: [{ name: "text", label: "结果", type: "text", required: true }],
    params: [
      { name: "cover", label: "载体文本(≥2词)", widget: { kind: "text", multiline: false }, default: "This is a confidential message" },
      { name: "password", label: "密码", widget: { kind: "text", multiline: false }, default: "" },
      { name: "encrypt", label: "加密 (AES-256-CTR)", widget: { kind: "toggle" }, default: true },
      { name: "integrity", label: "HMAC 完整性校验", widget: { kind: "toggle" }, default: false },
    ],
    cost: "cheap",
  },
  {
    id: "stegcloak_reveal",
    category: "隐写术",
    displayName: "StegCloak 解码",
    color: "#d946ef",
    inputs: [{ name: "text", label: "载体文本", type: "text", required: true }],
    outputs: [
      { name: "text", label: "秘密信息", type: "text", required: true },
      { name: "report", label: "分析", type: "text", required: false },
    ],
    params: [{ name: "password", label: "密码", widget: { kind: "text", multiline: false }, default: "" }],
    cost: "cheap",
  },
  {
    id: "whitespace_encode",
    category: "隐写术",
    displayName: "空白隐写编码",
    color: "#06b6d4",
    inputs: [{ name: "text", label: "秘密信息", type: "text", required: true }],
    outputs: [{ name: "text", label: "结果", type: "text", required: true }],
    params: [
      { name: "cover", label: "载体文本", widget: { kind: "text", multiline: false }, default: "" },
      { name: "zero", label: "0 = 字符", widget: { kind: "select", options: ["空格 (space)", "制表符 (tab)"] }, default: "空格 (space)" },
      { name: "msb", label: "高位在前 (MSB)", widget: { kind: "toggle" }, default: true },
    ],
    cost: "cheap",
  },
  {
    id: "whitespace_decode",
    category: "隐写术",
    displayName: "空白隐写解码",
    color: "#06b6d4",
    inputs: [{ name: "text", label: "载体文本", type: "text", required: true }],
    outputs: [
      { name: "text", label: "结果", type: "text", required: true },
      { name: "bits", label: "位串", type: "text", required: false },
      { name: "report", label: "分析", type: "text", required: false },
    ],
    params: [
      { name: "zero", label: "0 = 字符", widget: { kind: "select", options: ["空格 (space)", "制表符 (tab)"] }, default: "空格 (space)" },
      { name: "scope", label: "范围", widget: { kind: "select", options: ["行尾", "全部"] }, default: "行尾" },
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
  "SHA3-512", "Keccak-256", "RIPEMD-160", "BLAKE2b", "BLAKE2s", "Whirlpool", "SM3", "CRC32", "Adler-32",
];
const CYAN = "#06b6d4", SLATE = "#64748b", ROSE = "#f43f5e", TEAL = "#14b8a6";
const fmt = (name: string, label: string, def: string) =>
  sel(name, label, ["UTF8", "Hex", "Base64"], def);

pushDesc("hash", "哈希/摘要", "哈希计算", CYAN, anyIn(), [p("text", "摘要(hex)", "text")], [sel("algorithm", "算法", HASH_ALGOS, "SHA256")]);
pushDesc("hash_crack", "哈希/摘要", "哈希爆破", CYAN, [p("hash", "目标哈希(hex)", "text"), p("wordlist", "字典", "any")], [p("text", "明文", "text"), p("found", "命中", "bool", false), p("report", "信息", "text", false)], [sel("algorithm", "算法", HASH_ALGOS, "MD5"), txt("salt", "盐(可选)", ""), sel("saltMode", "加盐位置", ["无", "前缀", "后缀"], "无")]);
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
pushDesc("selector", "控制/逻辑", "选择器", AMBER, [], [p("value", "值", "text")], [txt("value", "值", "")]);
pushDesc("gate", "控制/逻辑", "条件门", AMBER, [p("value", "值", "any"), p("condition", "条件", "bool")], [p("output", "输出", "any"), p("passed", "已通过", "bool", false)], []);
pushDesc("string_match", "控制/逻辑", "字符串匹配", AMBER, [p("text", "输入", "text")], [p("result", "匹配", "bool"), p("text", "原文", "text", false)], [sel("operation", "判断", ["包含", "不包含", "等于", "不等于", "开头是", "结尾是", "正则匹配", "正则不匹配"], "包含"), txt("value", "匹配值 / 正则", ""), tog("ignoreCase", "忽略大小写", false)]);
pushDesc("range", "控制/逻辑", "数值范围", AMBER, [], [p("list", "序列", "stringList"), p("count", "数量", "number", false)], [num("start", "起始", -1000000, 1000000, 1, 0), num("end", "结束(不含)", -1000000, 1000000, 1, 10), num("step", "步长", -1000000, 1000000, 1, 1)]);
pushDesc("map", "控制/逻辑", "逐项映射", AMBER, [p("list", "列表", "stringList")], [p("list", "结果", "stringList")], [sel("op", "操作", XFORMS, "大写")]);
pushDesc("filter_list", "控制/逻辑", "列表过滤", AMBER, [p("list", "列表", "stringList")], [p("list", "结果", "stringList"), p("count", "数量", "number", false)], [txt("pattern", "正则", "."), sel("mode", "模式", ["保留匹配", "排除匹配"], "保留匹配")]);
pushDesc("join_list", "控制/逻辑", "列表合并", AMBER, [p("list", "列表", "stringList")], [p("text", "文本", "text")], [sel("sep", "分隔符", ["换行", "逗号", "空格", "无"], "换行")]);
pushDesc("iterate", "控制/逻辑", "迭代循环", AMBER, textIn(), [p("text", "结果", "text"), p("iterations", "迭代次数", "number", false), p("hit", "命中", "bool", false)], [sel("op", "操作", XFORMS, "Base64解码"), txt("until", "停止正则(可选)", "flag\\{[^}]*\\}"), num("max", "最大次数", 1, 100, 1, 16)]);

// CyberChef parity nodes (Batches A/B/C)
const BLUE = "#3b82f6";
const t2t = (id: string, cat: string, name: string, color: string, params: ParamSpec[]) =>
  pushDesc(id, cat, name, color, textIn(), textOut(), params);
const fmt3 = (name: string, label: string, def: string) => sel(name, label, ["UTF8", "Hex", "Base64"], def);
const blockCipher = (extra: ParamSpec[]): ParamSpec[] => [
  ...extra,
  txt("key", "密钥", ""), fmt3("keyFormat", "密钥格式", "Hex"),
  txt("iv", "IV", ""), fmt3("ivFormat", "IV 格式", "Hex"),
  fmt3("inputFormat", "输入格式", "UTF8"), sel("outputFormat", "输出格式", ["Hex", "Base64", "UTF8"], "Hex"),
];
const streamCipher = (extra: ParamSpec[]): ParamSpec[] => [
  ...extra,
  txt("key", "密钥", ""), fmt3("keyFormat", "密钥格式", "Hex"),
  txt("nonce", "Nonce", ""), fmt3("nonceFormat", "Nonce 格式", "Hex"),
  fmt3("inputFormat", "输入格式", "UTF8"), sel("outputFormat", "输出格式", ["Hex", "Base64", "UTF8"], "Hex"),
];

t2t("change_case", "文本处理", "大小写转换", TEAL, [sel("mode", "模式", ["大写", "小写", "词首大写", "句首大写", "交换大小写"], "大写")]);
t2t("remove_whitespace", "文本处理", "去除空白", TEAL, [sel("mode", "去除", ["全部空白", "空格", "换行", "制表符", "非可见字符"], "全部空白")]);
t2t("sort_lines", "文本处理", "行排序", TEAL, [sel("order", "顺序", ["字母升序", "字母降序", "数字升序", "长度升序", "反转"], "字母升序")]);
t2t("unique_lines", "文本处理", "行去重", TEAL, [tog("count", "统计出现次数", false)]);
t2t("substring", "文本处理", "截取子串", TEAL, [num("start", "起始位置", 0, 1000000, 1, 0), num("length", "长度(0=到末尾)", 0, 1000000, 1, 0)]);
t2t("regex_replace", "文本处理", "正则替换", TEAL, [txt("pattern", "正则", ""), txt("replacement", "替换为", ""), tog("global", "全部替换", true)]);
t2t("pad_lines", "文本处理", "行填充", TEAL, [num("width", "目标宽度", 0, 1000, 1, 8), txt("char", "填充字符", " "), sel("side", "方向", ["右侧", "左侧"], "右侧")]);

t2t("caesar", "加密解密", "凯撒密码", ROSE, [num("amount", "位移量", 0, 25, 1, 3)]);
t2t("rail_fence_encode", "加密解密", "栅栏密码加密", ROSE, [num("rails", "栏数", 2, 100, 1, 3)]);
t2t("rail_fence_decode", "加密解密", "栅栏密码解密", ROSE, [num("rails", "栏数", 2, 100, 1, 3)]);
pushDesc("des", "加密解密", "DES / 3DES", ROSE, textIn(), decOut(), blockCipher([sel("operation", "操作", ["加密", "解密"], "加密"), sel("mode", "模式", ["CBC", "ECB"], "CBC")]));
pushDesc("blowfish", "加密解密", "Blowfish", ROSE, textIn(), decOut(), blockCipher([sel("operation", "操作", ["加密", "解密"], "加密"), sel("mode", "模式", ["CBC", "ECB"], "CBC")]));
pushDesc("chacha20", "加密解密", "ChaCha20", ROSE, textIn(), decOut(), streamCipher([sel("variant", "变体", ["ChaCha20", "XChaCha20"], "ChaCha20")]));
pushDesc("salsa20", "加密解密", "Salsa20", ROSE, textIn(), decOut(), streamCipher([]));

t2t("morse_encode", "编码/加密", "摩尔斯编码", BLUE, []);
t2t("morse_decode", "编码/加密", "摩尔斯解码", BLUE, []);
t2t("bacon_encode", "编码/加密", "培根密码编码", BLUE, []);
t2t("bacon_decode", "编码/加密", "培根密码解码", BLUE, []);
t2t("a1z26_encode", "编码/加密", "A1Z26 编码", BLUE, [sel("delimiter", "分隔符", ["空格", "逗号", "短横"], "空格")]);
t2t("a1z26_decode", "编码/加密", "A1Z26 解码", BLUE, []);
t2t("html_entity_encode", "编码/加密", "HTML 实体编码", BLUE, [sel("mode", "范围", ["仅特殊字符", "全部非ASCII"], "仅特殊字符")]);
t2t("html_entity_decode", "编码/加密", "HTML 实体解码", BLUE, []);
t2t("unicode_escape", "编码/加密", "Unicode 转义", BLUE, [sel("mode", "范围", ["仅非ASCII", "全部"], "仅非ASCII")]);
t2t("unicode_unescape", "编码/加密", "Unicode 反转义", BLUE, []);
pushDesc("to_hexdump", "编码/加密", "转 Hexdump", BLUE, anyIn(), textOut(), []);
pushDesc("from_hexdump", "编码/加密", "Hexdump 转字节", BLUE, textIn(), decOut(), []);
pushDesc("bitwise", "编码/加密", "位运算", BLUE, anyIn(), decOut(), [sel("operation", "运算", ["XOR", "AND", "OR", "NOT", "左移", "右移", "循环左移", "循环右移"], "XOR"), txt("key", "密钥(Hex)", ""), num("amount", "位数", 0, 7, 1, 1)]);

pushDesc("to_octal", "进制转换", "转八进制", SLATE, anyIn(), textOut(), [sel("delimiter", "分隔符", ["空格", "无"], "空格")]);
pushDesc("from_octal", "进制转换", "八进制转文本", SLATE, textIn(), decOut(), []);

pushDesc("entropy", "工具/分析", "香农熵", AMBER, anyIn(), [p("entropy", "熵", "number"), p("text", "说明", "text", false)], []);
pushDesc("password_crack", "工具/分析", "通用口令爆破", AMBER, [p("data", "目标输入", "any"), p("wordlist", "字典", "any")], [p("password", "命中口令", "text"), p("text", "解出文本", "text", false), p("bytes", "解出字节", "bytes", false), p("found", "命中", "bool", false), p("report", "信息", "text", false)], [txt("node", "目标节点 id", "cloacked_pixel_extract"), txt("passwordParam", "口令参数名", "password"), sel("success", "成功判据", ["无报错(能解出)", "正则命中", "可打印文本"], "无报错(能解出)"), txt("pattern", "正则(正则命中判据)", "flag\\{"), txt("checkPort", "检查的输出端口(留空自动)", ""), txt("inputPort", "目标输入端口(留空自动)", ""), txt("extraParams", "目标额外参数(JSON)", "")]);
pushDesc("http_request", "工具/分析", "HTTP 请求", "#6366f1", [p("body", "请求体(可选)", "any", false)], [p("text", "响应体", "text"), p("bytes", "响应字节", "bytes", false), p("status", "状态码", "number", false), p("headers", "响应头", "text", false), p("report", "信息", "text", false)], [sel("method", "方法", ["GET", "POST", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"], "GET"), txt("url", "URL", "https://httpbin.org/get"), sel("httpVersion", "HTTP 版本", ["自动", "1.1", "2.0"], "自动"), txt("userAgent", "User-Agent", "Mozilla/5.0 … Chrome/120"), txt("cookies", "Cookie", ""), txt("headers", "请求头(每行 K: V)", ""), txt("contentType", "Content-Type", ""), txt("body", "请求体(无输入时用)", ""), num("timeout", "超时(秒)", 1, 300, 1, 15), tog("followRedirects", "跟随重定向", true), tog("insecure", "忽略证书错误", false)]);
t2t("char_frequency", "工具/分析", "字符频率", AMBER, []);
t2t("defang", "工具/分析", "Defang/Refang", AMBER, [sel("operation", "操作", ["defang", "refang"], "defang")]);
pushDesc("jwt_decode", "工具/分析", "JWT 解码", CYAN, textIn(), [p("text", "载荷", "text"), p("payload", "payload", "text", false), p("header", "header", "text", false)], []);
pushDesc("jwt_crack", "工具/分析", "JWT 密钥爆破", CYAN, [p("text", "JWT", "text"), p("wordlist", "字典", "any")], [p("text", "密钥", "text"), p("found", "命中", "bool", false), p("report", "信息", "text", false)], [sel("algorithm", "算法", ["自动", "HS256", "HS384", "HS512"], "自动")]);

pushDesc("compress", "压缩包", "压缩", AMBER, anyIn(), [p("hex", "hex", "text"), p("bytes", "字节", "bytes", false)], [sel("format", "格式", ["Gzip", "Zlib", "Raw Deflate"], "Gzip")]);
pushDesc("archive_list", "压缩包", "压缩包文件列表", AMBER, [p("archive", "压缩包字节", "any")], [p("text", "列表", "text"), p("files", "文件名", "stringList", false), p("count", "条目数", "number", false), p("encrypted", "含加密", "bool", false)], []);
pushDesc("zip_repair", "压缩包", "压缩包伪加密修复", AMBER, [p("data", "ZIP", "any")], [p("bytes", "修复后字节", "bytes"), p("report", "分析", "text", false)], [tog("clearStrong", "同时清强加密位(bit6)", false)]);
t2t("json_format", "工具/分析", "JSON 格式化", CYAN, [sel("operation", "操作", ["美化", "压缩"], "美化")]);
t2t("substitution", "加密解密", "替换密码", ROSE, [txt("from", "明文字母表", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"), txt("to", "密文字母表", "")]);
t2t("braille_encode", "编码/加密", "盲文编码", BLUE, []);
t2t("braille_decode", "编码/加密", "盲文解码", BLUE, []);
t2t("from_timestamp", "工具/分析", "时间戳转日期", CYAN, []);
t2t("to_timestamp", "工具/分析", "日期转时间戳", CYAN, []);
pushDesc("rsa_params", "加密解密", "RSA 参数计算", ROSE, [], [p("text", "摘要", "text"), p("n", "n", "text", false), p("phi", "φ(n)", "text", false), p("d", "d", "text", false)], [txt("p", "素数 p", ""), txt("q", "素数 q", ""), txt("e", "公钥指数 e", "65537")]);
pushDesc("rsa_decrypt", "加密解密", "RSA 解密", ROSE, textIn(), [p("text", "明文", "text"), p("int", "整数 m", "text", false), p("hex", "hex", "text", false), p("bytes", "字节", "bytes", false)], [txt("n", "模数 n", ""), txt("d", "私钥 d", ""), txt("p", "素数 p", ""), txt("q", "素数 q", ""), txt("e", "e", "65537")]);
t2t("bifid_encode", "加密解密", "Bifid 加密", ROSE, [txt("keyword", "关键词", "")]);
t2t("bifid_decode", "加密解密", "Bifid 解密", ROSE, [txt("keyword", "关键词", "")]);
t2t("playfair_encode", "加密解密", "Playfair 加密", ROSE, [txt("keyword", "关键词", "")]);
t2t("playfair_decode", "加密解密", "Playfair 解密", ROSE, [txt("keyword", "关键词", "")]);
pushDesc("detect_file_type", "工具/分析", "文件类型识别", AMBER, anyIn(), [p("text", "结果", "text"), p("type", "类型", "text", false), p("ext", "后缀名", "text", false)], []);
pushDesc("file_carve", "工具/分析", "文件雕刻(binwalk)", AMBER, anyIn(), [p("text", "命中清单", "text"), p("bytes", "抽取的文件", "bytes", false), p("count", "内嵌数", "number", false)], [num("index", "抽取第几个(0基)", 0, 64, 1, 0), txt("signatures", "限定后缀(逗号分隔,空=全部)", "")]);
pushDesc("extract", "工具/分析", "信息提取", AMBER, textIn(), [p("text", "匹配(每行一个)", "text"), p("matches", "列表", "stringList", false), p("count", "数量", "number", false)], [sel("kind", "提取类型", ["IPv4", "IPv6", "邮箱", "URL", "MAC地址", "域名", "flag", "Base64块", "Hex串"], "IPv4"), tog("unique", "去重", true)]);
pushDesc("rotate_bytes", "工具/分析", "位旋转 (ROL/ROR)", AMBER, anyIn(), [p("bytes", "字节", "bytes"), p("hex", "hex", "text", false), p("text", "文本", "text", false)], [sel("direction", "方向", ["左(ROL)", "右(ROR)"], "左(ROL)"), num("amount", "位数", 0, 64, 1, 1), tog("carry", "跨字节进位", false)]);
t2t("to_charcode", "进制转换", "字符转码点", "#6366f1", [sel("base", "进制", ["16", "10", "8", "2"], "16"), sel("delimiter", "分隔符", ["空格", "逗号", "换行", "分号"], "空格")]);
t2t("from_charcode", "进制转换", "码点转字符", "#6366f1", [sel("base", "进制", ["16", "10", "8", "2"], "16"), sel("delimiter", "分隔符", ["空格", "逗号", "换行", "分号"], "空格")]);
pushDesc("quoted_printable_encode", "字符编码", "Quoted-Printable 编码", "#6366f1", anyIn(), textOut(), []);
pushDesc("quoted_printable_decode", "字符编码", "Quoted-Printable 解码", "#6366f1", textIn(), decOut(), []);
// Batch G — cold hashes / classical / decompress / forensics / PGP
pushDesc("bcrypt", "哈希/摘要", "bcrypt", CYAN, [p("text", "口令", "text")], [p("text", "结果", "text"), p("result", "匹配", "bool", false)], [sel("operation", "操作", ["哈希", "校验"], "哈希"), num("cost", "代价(4-15)", 4, 15, 1, 10), txt("hash", "校验目标 hash", "")]);
t2t("enigma", "加密解密", "Enigma 机", ROSE, [txt("rotors", "转子(左→右)", "I II III"), sel("reflector", "反射器", ["B", "C"], "B"), txt("ring", "环设置(3字母)", "AAA"), txt("position", "初始位置(3字母)", "AAA"), txt("plugboard", "插线板(如 AB CD)", "")]);
t2t("adfgvx", "加密解密", "ADFGVX 密码", ROSE, [sel("operation", "操作", ["加密", "解密"], "加密"), sel("variant", "变体", ["ADFGVX (6×6)", "ADFGX (5×5)"], "ADFGVX (6×6)"), txt("keyword", "转置关键词", "SECRET"), txt("square", "方阵关键词(可空)", "")]);
pushDesc("exif_extract", "工具/分析", "EXIF 信息", AMBER, anyIn(), [p("text", "元数据", "text"), p("fields", "字段", "stringList", false), p("count", "数量", "number", false)], []);
pushDesc("lsb_extract", "隐写术", "LSB 提取", "#a855f7", anyIn(), [p("text", "文本", "text"), p("bytes", "字节", "bytes", false), p("hex", "hex", "text", false)], [txt("channels", "通道顺序 (R/G/B/A)", "RGB"), num("bit", "位平面 (0=最低位)", 0, 7, 1, 0), tog("msbFirst", "高位在前打包", true)]);
pushDesc("text_blind_watermark", "隐写术", "文本盲水印提取", "#a855f7", [p("text", "含水印文本", "text")], [p("text", "水印", "text")], [txt("password", "密码", ""), sel("pwType", "密码类型", ["自动", "整数", "字符串"], "自动"), sel("variant", "变体", ["自动", "DEL(chr127)", "零宽字符"], "自动")]);
pushDesc("guofei_bwm_extract", "隐写术", "盲水印提取(guofei9987)", "#a855f7", [p("data", "含水印图", "any")], [p("text", "文本水印", "text", false), p("image", "图片水印", "image", false), p("report", "说明", "text", false)], [sel("mode", "水印类型", ["文本", "图片"], "文本"), num("wmLength", "水印位数(文本)", 0, 1000000, 1, 0), num("wmWidth", "水印宽(图片)", 0, 4096, 1, 64), num("wmHeight", "水印高(图片)", 0, 4096, 1, 64), num("pwWm", "水印密码 password_wm", 0, 4294967295, 1, 1), num("pwImg", "图密码 password_img", 0, 4294967295, 1, 1)]);
pushDesc("imagein_extract", "隐写术", "imageIN 文件提取", "#a855f7", [p("data", "图片", "any")], [p("text", "文本预览", "text"), p("bytes", "文件字节", "bytes", false), p("filename", "文件名", "text", false), p("report", "信息", "text", false)], [sel("channels", "通道", ["全部(BGR)", "B(蓝)", "G(绿)", "R(红)"], "全部(BGR)"), num("depth", "深度(0=自动识别)", 0, 8, 1, 0)]);
pushDesc("imagein_embed", "隐写术", "imageIN 文件嵌入", "#a855f7", [p("data", "载体图片", "any"), p("file", "要嵌入的文件", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false), p("report", "信息", "text", false)], [txt("filename", "记录的文件名", "secret.bin"), sel("channels", "通道", ["全部(BGR)", "B(蓝)", "G(绿)", "R(红)"], "全部(BGR)"), num("depth", "深度(0=自动)", 0, 8, 1, 0)]);
pushDesc("file_output", "输入输出", "文件输出", SLATE, [p("data", "数据", "any")], [p("path", "保存路径", "text")], [txt("filename", "文件名", "output.bin")]);
// third-party image-stego tools
const MASK_PARAMS = () => [num("mixCount", "混合位数(1-5)", 1, 5, 1, 2), num("charSize", "字符位数", 8, 32, 1, 16), num("lengthSize", "长度位数", 8, 32, 1, 24)];
pushDesc("cloacked_pixel_extract", "隐写术", "cloacked-pixel 提取", "#a855f7", [p("data", "图片", "any")], [p("text", "文本", "text"), p("bytes", "字节", "bytes", false), p("hex", "hex", "text", false)], [txt("password", "密码", "")]);
pushDesc("cloacked_pixel_embed", "隐写术", "cloacked-pixel 嵌入", "#a855f7", [p("data", "载体图片", "any"), p("file", "载荷", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], [txt("password", "密码", "")]);
pushDesc("imagemask_text_extract", "隐写术", "ImageMask 文本提取", "#a855f7", [p("data", "图片", "any")], [p("text", "文本", "text")], MASK_PARAMS());
pushDesc("imagemask_file_extract", "隐写术", "ImageMask 文件提取", "#a855f7", [p("data", "图片", "any")], [p("bytes", "文件字节", "bytes"), p("filename", "文件名", "text", false), p("text", "文本预览", "text", false)], MASK_PARAMS());
pushDesc("imagemask_text_embed", "隐写术", "ImageMask 文本嵌入", "#a855f7", [p("data", "载体图片", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], [txt("text", "要隐写的文本", ""), ...MASK_PARAMS()]);
pushDesc("imagemask_file_embed", "隐写术", "ImageMask 文件嵌入", "#a855f7", [p("data", "载体图片", "any"), p("file", "要嵌入的文件", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], [txt("filename", "记录的文件名", "secret.bin"), ...MASK_PARAMS()]);
pushDesc("bmp_padding_extract", "隐写术", "BMP 填补字节提取", "#a855f7", [p("data", "BMP", "any")], [p("text", "文本", "text"), p("bytes", "字节", "bytes", false), p("hex", "hex", "text", false), p("report", "信息", "text", false)], []);
pushDesc("bmp_padding_embed", "隐写术", "BMP 填补字节嵌入", "#a855f7", [p("data", "BMP", "any"), p("file", "载荷", "any")], [p("bytes", "BMP字节", "bytes"), p("text", "信息", "text", false)], []);
pushDesc("psimage_extract", "隐写术", "Invoke-PSImage 提取", "#a855f7", [p("data", "图片", "any")], [p("text", "PS 脚本", "text"), p("bytes", "字节", "bytes", false)], [tog("trim", "截去尾部随机填充", true)]);
pushDesc("psimage_embed", "隐写术", "Invoke-PSImage 嵌入", "#a855f7", [p("data", "载体图片", "any"), p("file", "脚本/载荷", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], []);
pushDesc("stegpy_extract", "隐写术", "stegpy 提取", "#a855f7", [p("data", "图片", "any")], [p("text", "文本", "text"), p("bytes", "字节", "bytes", false), p("filename", "文件名", "text", false)], []);
pushDesc("stegpy_embed", "隐写术", "stegpy 嵌入", "#a855f7", [p("data", "载体图片", "any"), p("file", "载荷", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], [txt("filename", "文件名(空=文本模式)", ""), sel("bits", "每字节位数", ["1", "2", "4"], "2")]);
pushDesc("braintools_decode", "隐写术", "BrainTools (Brainfuck 图)", "#a855f7", [p("data", "图片", "any")], [p("text", "Brainfuck 源码", "text"), p("output", "运行输出", "text", false), p("bytes", "输出字节", "bytes", false)], [sel("mode", "模式", ["Braincopter", "Brainloller"], "Braincopter"), tog("run", "执行程序", true), txt("input", "标准输入(可选)", "")]);
pushDesc("brainfuck", "编码/加密", "Brainfuck 解释器", "#a855f7", [p("text", "源码", "text"), p("stdin", "标准输入", "any", false)], [p("text", "输出", "text"), p("bytes", "输出字节", "bytes", false)], [sel("dialect", "方言", ["Brainfuck", "Ook!"], "Brainfuck"), txt("input", "标准输入(可选)", "")]);
pushDesc("pixeljihad_extract", "隐写术", "PixelJihad 提取", "#a855f7", [p("data", "图片", "any")], [p("text", "文本", "text"), p("json", "原始JSON", "text", false)], [txt("password", "密码", "")]);
pushDesc("pixeljihad_embed", "隐写术", "PixelJihad 嵌入", "#a855f7", [p("data", "载体图片", "any")], [p("image", "图片", "image"), p("bytes", "PNG字节", "bytes", false)], [txt("message", "要隐写的文本", ""), txt("password", "密码(暂仅支持空)", "")]);
pushDesc("pgp_dearmor", "加密解密", "PGP 解甲(Dearmor)", ROSE, textIn(), [p("bytes", "字节", "bytes"), p("hex", "hex", "text", false), p("type", "块类型", "text", false), p("crcOk", "CRC 校验", "bool", false)], []);
pushDesc("pgp_enarmor", "加密解密", "PGP 装甲(Enarmor)", ROSE, anyIn(), textOut(), [sel("blockType", "块类型", ["MESSAGE", "PUBLIC KEY BLOCK", "PRIVATE KEY BLOCK", "SIGNATURE"], "MESSAGE")]);
pushDesc("pgp_decrypt", "加密解密", "PGP 解密", ROSE, [p("text", "PGP 消息", "text"), p("key", "私钥(armored)", "text")], [p("text", "明文", "text"), p("bytes", "字节", "bytes", false), p("hex", "hex", "text", false)], [txt("passphrase", "口令(可空)", "")]);
pushDesc("cryptojs_aes", "加密解密", "CryptoJS AES 解密", ROSE, [p("text", "密文(base64)", "text"), p("wordlist", "字典(可选爆破)", "any", false)], [p("text", "明文", "text"), p("bytes", "字节", "bytes", false), p("report", "信息", "text", false)], [txt("password", "口令", ""), sel("keySize", "密钥长度", ["128", "192", "256"], "256"), sel("outputFormat", "输出格式", ["UTF8", "Hex", "Base64"], "UTF8")]);

// 图像处理
const IMGF = "#d946ef", IMGI = "#6366f1", IMGT = "#14b8a6";
const iin = () => [p("data", "图片", "any")];
const iout = () => [p("image", "图片", "image"), p("bytes", "字节", "bytes", false)];
pushDesc("image_blend", "图像处理", "图像混合", IMGF, [p("a", "图片 A", "any"), p("b", "图片 B", "any")], iout(), [sel("mode", "模式", ["异或", "相加", "相减", "差值", "相乘", "变亮", "变暗", "叠加(alpha混合)", "屏幕", "溶解"], "异或"), num("alpha", "alpha", 0, 1, 0.05, 0.5), sel("align", "尺寸对齐", ["裁剪到较小", "缩放B到A"], "裁剪到较小")]);
pushDesc("image_concat", "图像处理", "图像拼接", IMGF, [p("a", "图片 A", "any"), p("b", "图片 B", "any")], iout(), [sel("direction", "方向", ["水平", "垂直"], "水平")]);
pushDesc("channel_merge", "图像处理", "通道合并", IMGF, [p("r", "R", "any"), p("g", "G", "any"), p("b", "B", "any"), p("a", "A", "any", false)], iout(), []);
pushDesc("channel_extract", "图像处理", "通道提取", IMGF, iin(), iout(), [sel("channel", "通道", ["R", "G", "B", "A", "灰度"], "R"), sel("output", "输出", ["灰度图", "仅该通道"], "灰度图")]);
pushDesc("channel_split", "图像处理", "通道分离", IMGF, iin(), [p("r", "R", "image"), p("g", "G", "image"), p("b", "B", "image"), p("a", "A", "image", false)], []);
pushDesc("channel_swap", "图像处理", "通道交换", IMGF, iin(), iout(), [sel("order", "顺序", ["RGB", "RBG", "GRB", "GBR", "BRG", "BGR"], "BGR")]);
pushDesc("bit_plane", "图像处理", "位平面提取", IMGF, iin(), iout(), [sel("channel", "通道", ["R", "G", "B", "A", "灰度"], "R"), num("bit", "位", 0, 7, 1, 0)]);
pushDesc("grayscale", "图像处理", "灰度化", IMGI, iin(), iout(), []);
pushDesc("image_invert", "图像处理", "反色", IMGI, iin(), iout(), []);
pushDesc("threshold", "图像处理", "二值化", IMGI, iin(), iout(), [num("threshold", "阈值", 0, 255, 1, 128), tog("auto", "自动(Otsu)", false), tog("invert", "反转", false)]);
pushDesc("brightness_contrast", "图像处理", "亮度对比度", IMGI, iin(), iout(), [num("brightness", "亮度", -255, 255, 1, 0), num("contrast", "对比度", -100, 100, 1, 0)]);
pushDesc("gamma", "图像处理", "伽马校正", IMGI, iin(), iout(), [num("gamma", "γ", 0.1, 5, 0.1, 1)]);
pushDesc("hist_equalize", "图像处理", "直方图均衡", IMGI, iin(), iout(), []);
pushDesc("edge_detect", "图像处理", "边缘检测", IMGI, iin(), iout(), []);
pushDesc("image_xor", "图像处理", "常数异或", IMGI, iin(), iout(), [txt("key", "密钥", "ff"), sel("keyFormat", "格式", ["Hex", "整数"], "Hex")]);
pushDesc("image_transform", "图像处理", "旋转翻转", IMGT, iin(), iout(), [sel("op", "操作", ["旋转90°", "旋转180°", "旋转270°", "水平翻转", "垂直翻转"], "旋转90°")]);
pushDesc("image_crop", "图像处理", "裁剪", IMGT, iin(), iout(), [num("x", "X", 0, 100000, 1, 0), num("y", "Y", 0, 100000, 1, 0), num("width", "宽", 1, 100000, 1, 100), num("height", "高", 1, 100000, 1, 100)]);
pushDesc("image_resize", "图像处理", "缩放", IMGT, iin(), iout(), [num("width", "宽", 1, 10000, 1, 256), num("height", "高", 1, 10000, 1, 256), tog("keepAspect", "保持宽高比", false)]);
pushDesc("image_info", "图像处理", "图像信息", IMGT, iin(), [p("text", "信息", "text"), p("width", "宽", "number", false), p("height", "高", "number", false)], []);
pushDesc("image_convert", "图像处理", "格式转换", IMGT, iin(), iout(), [sel("format", "格式", ["PNG", "JPEG", "BMP", "GIF"], "PNG")]);
pushDesc("colorspace_extract", "图像处理", "色彩空间分量", IMGF, iin(), iout(), [sel("space", "色彩空间", ["HSV", "YCbCr"], "HSV"), sel("component", "分量", ["分量1(H/Y)", "分量2(S/Cb)", "分量3(V/Cr)"], "分量1(H/Y)")]);
pushDesc("image_diff", "图像处理", "图像差异", IMGF, [p("a", "图片 A", "any"), p("b", "图片 B", "any")], [p("image", "差异图", "image"), p("bytes", "字节", "bytes", false), p("count", "差异像素数", "number", false)], [num("threshold", "阈值", 0, 255, 1, 16)]);
pushDesc("gif_frame", "图像处理", "GIF 取帧", IMGF, [p("data", "GIF", "any")], [p("image", "图片", "image"), p("bytes", "字节", "bytes", false), p("count", "帧数", "number", false)], [num("index", "帧序号", 0, 100000, 1, 0)]);
pushDesc("gif_sprite", "图像处理", "GIF 拼帧", IMGF, [p("data", "GIF", "any")], [p("image", "图片", "image"), p("bytes", "字节", "bytes", false), p("count", "帧数", "number", false)], [num("columns", "每行帧数", 1, 64, 1, 8)]);
pushDesc("gif_timing", "图像处理", "GIF 帧时序解码", IMGF, [p("data", "GIF", "any")], [p("text", "结果", "text"), p("bytes", "字节", "bytes", false), p("delays", "帧时长(厘秒)", "text", false)], [sel("mode", "映射", ["原始数字", "字节/ASCII", "二进制"], "字节/ASCII"), num("threshold", "二进制阈值(厘秒)", 0, 1000, 1, 5)]);
pushDesc("dft_spectrum", "图像处理", "频谱 (DFT)", IMGI, iin(), [p("image", "频谱图", "image"), p("bytes", "字节", "bytes", false)], []);
pushDesc("connected_components", "图像处理", "连通域标记", IMGF, iin(), [p("image", "标记图", "image"), p("bytes", "字节", "bytes", false), p("count", "区域数", "number", false)], [num("threshold", "二值阈值", 0, 255, 1, 128)]);
pushDesc("morphology", "图像处理", "形态学", IMGI, iin(), iout(), [sel("op", "运算", ["膨胀", "腐蚀", "开运算", "闭运算"], "膨胀"), num("size", "核半径", 0, 50, 1, 1), num("threshold", "二值阈值", 0, 255, 1, 128)]);
pushDesc("template_match", "图像处理", "模板匹配", IMGT, [p("image", "图片", "any"), p("template", "模板", "any")], [p("image", "标记图", "image"), p("text", "结果", "text", false), p("x", "X", "number", false), p("y", "Y", "number", false)], []);
pushDesc("png_fix", "图像处理", "PNG 宽高修复", IMGT, [p("data", "PNG", "any")], [p("image", "修复后", "image"), p("bytes", "字节", "bytes", false), p("report", "分析", "text", false)], [sel("mode", "模式", ["自动", "CRC 爆破", "手动"], "自动"), num("max", "CRC爆破上限(像素)", 1, 65535, 1, 8192), num("width", "宽(手动,0=不改)", 0, 1000000, 1, 0), num("height", "高(手动,0=不改)", 0, 1000000, 1, 0)]);
pushDesc("jpg_fix", "图像处理", "JPG 宽高修复", IMGT, [p("data", "JPEG", "any")], [p("image", "修复后", "image"), p("bytes", "字节", "bytes", false), p("report", "分析", "text", false)], [sel("mode", "模式", ["自动", "手动"], "自动"), num("width", "宽(手动,0=不改)", 0, 100000, 1, 0), num("height", "高(手动,0=不改)", 0, 100000, 1, 0)]);
pushDesc("bmp_fix", "图像处理", "BMP 宽高修复", IMGT, [p("data", "BMP", "any")], [p("image", "修复后", "image"), p("bytes", "字节", "bytes", false), p("report", "分析", "text", false)], [sel("mode", "模式", ["自动", "手动"], "自动"), num("width", "宽(手动,0=不改)", 0, 1000000, 1, 0), num("height", "高(手动,0=不改)", 0, 1000000, 1, 0)]);
pushDesc("blind_watermark", "图像处理", "盲水印 (FFT)", IMGI, iin(), [p("image", "频谱/水印", "image"), p("bytes", "字节", "bytes", false)], [sel("mode", "模式", ["Java-BlindWatermark", "FFT(Multiplier)", "FFT(fftshiftMultiplier)", "FFT(Normalization)", "FFT(fftshift_Normalization)"], "Java-BlindWatermark"), sel("channel", "通道", ["灰度", "R", "G", "B"], "灰度"), num("multiplier", "乘数(Multiplier 模式)", 0, 100000, 0.1, 1)]);
pushDesc("blind_watermark_dual", "图像处理", "两图盲水印", IMGI, [p("a", "原图", "any"), p("b", "含水印图", "any")], [p("image", "水印", "image"), p("bytes", "字节", "bytes", false)], [sel("mode", "模式", ["频率盲水印", "异或(XOR)", "差值(放大)"], "频率盲水印"), num("seed", "随机种子(频率)", 0, 4294967295, 1, 20160930), num("alpha", "alpha(频率)", 0.1, 100, 0.1, 3), tog("oldseed", "Python2 随机(oldseed)", false), num("amplify", "放大倍数(差值)", 1, 64, 1, 8)]);
pushDesc("bits_to_image", "图像处理", "01 转图像", IMGT, [p("text", "0/1 文本", "text")], iout(), [sel("mode", "布局", ["自动", "按行", "按宽度"], "自动"), num("width", "宽度(按宽度,0=自动)", 0, 100000, 1, 0), tog("invert", "取反(1=白)", false), num("scale", "放大倍数", 1, 64, 1, 1)]);
pushDesc("image_to_bits", "图像处理", "图像转 01", IMGT, iin(), [p("text", "0/1 文本", "text"), p("width", "宽", "number", false), p("height", "高", "number", false)], [num("threshold", "阈值", 0, 255, 1, 128), tog("otsu", "自动阈值(Otsu)", false), tog("invert", "取反(亮=1)", false), tog("rows", "按行换行", true)]);

// Audio stego
const AUDC = "#d946ef";
pushDesc("audio_info", "音频处理", "音频信息", AUDC, [p("data", "音频", "any")], [p("text", "信息", "text"), p("sampleRate", "采样率", "number", false), p("channels", "声道数", "number", false), p("duration", "时长(秒)", "number", false)], []);
pushDesc("audio_spectrogram", "音频处理", "音频频谱图", AUDC, [p("data", "音频", "any")], [p("image", "频谱图", "image"), p("bytes", "PNG字节", "bytes", false), p("report", "参数", "text", false)], [sel("channel", "声道", ["混合", "左声道", "右声道"], "混合"), sel("fftSize", "FFT 窗口", ["512", "1024", "2048", "4096"], "1024"), sel("overlap", "重叠", ["50%", "75%", "87.5%"], "75%"), sel("colormap", "配色", ["彩色", "灰度"], "彩色"), num("dynamicRange", "动态范围(dB)", 30, 120, 5, 80)]);
pushDesc("wav_lsb", "音频处理", "WAV LSB 提取", AUDC, [p("data", "音频", "any")], [p("bytes", "字节", "bytes"), p("text", "文本", "text", false), p("hex", "hex预览", "text", false)], [num("bits", "每采样取LSB位数", 1, 8, 1, 1), sel("channel", "声道", ["全部(交错)", "左声道", "右声道"], "全部(交错)"), sel("bitOrder", "位序", ["MSB优先", "LSB优先"], "MSB优先")]);
pushDesc("dtmf_decode", "音频处理", "DTMF 拨号音解码", AUDC, [p("data", "音频", "any")], [p("text", "数字", "text"), p("report", "说明", "text", false)], []);
pushDesc("deepsound_extract", "音频处理", "DeepSound 提取", AUDC, [p("data", "音频", "any")], [p("bytes", "首个文件", "bytes"), p("text", "文本", "text", false), p("filename", "文件名", "text", false), p("report", "分析", "text", false)], [txt("password", "密码(加密时填)", "")]);
pushDesc("pixel_extract", "图像处理", "提取像素值", IMGT, iin(), [p("text", "数值", "text"), p("width", "宽", "number", false), p("height", "高", "number", false)], [sel("channel", "通道", ["灰度", "R", "G", "B", "A", "RGB", "RGBA"], "灰度"), sel("base", "进制", ["十进制", "十六进制"], "十进制"), sel("sep", "分隔符", ["空格", "逗号"], "空格"), tog("rows", "按行换行", true)]);
pushDesc("values_to_image", "图像处理", "像素值转图像", IMGT, [p("text", "数值", "text")], iout(), [sel("channels", "通道数", ["灰度(1)", "RGB(3)", "RGBA(4)"], "灰度(1)"), num("width", "宽度(0=自动)", 0, 100000, 1, 0), sel("base", "进制", ["十进制", "十六进制"], "十进制")]);

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
