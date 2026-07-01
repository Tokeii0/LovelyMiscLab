import {
  Binary,
  Bomb,
  EyeOff,
  FileArchive,
  Fingerprint,
  Hash,
  type LucideIcon,
  Network,
  QrCode,
  Lock,
  Regex,
  Repeat,
  RotateCw,
  ScanLine,
  Wand2,
} from "lucide-react";

/** A node inside a template. `key` is template-local; real ids are minted on load. */
export interface TemplateNode {
  key: string;
  descriptorId: string;
  position: { x: number; y: number };
  params?: Record<string, unknown>;
}

export interface TemplateEdge {
  from: { node: string; port: string };
  to: { node: string; port: string };
}

export interface Template {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: LucideIcon;
  nodes: TemplateNode[];
  edges: TemplateEdge[];
}

// Horizontal lane layout helper.
const X = (i: number) => 40 + i * 240;
const Y = 150;

export const TEMPLATE_CATEGORIES = ["综合演示", "编码解码", "文本处理", "控制/流程", "密码学", "取证/文件"] as const;

export const TEMPLATES: Template[] = [
  {
    id: "showcase-multi-branch",
    name: "综合演示 · 多分支解密重组",
    description:
      "四条编码分支（Base64 / Hex / ROT13 / 反转）各自解出 flag 的一个片段，合并重组后正则提取，并分出二维码、可读性评分与相等校验三路分析。用于演示节点图的分支与合流。",
    category: "综合演示",
    icon: Network,
    nodes: [
      // Source fragments (col 0)
      { key: "in_b64", descriptorId: "text_input", position: { x: 40, y: 40 }, params: { text: "ZmxhZ3s=" } },
      { key: "in_hex", descriptorId: "text_input", position: { x: 40, y: 190 }, params: { text: "6d756c74695f" } },
      { key: "in_rot", descriptorId: "text_input", position: { x: 40, y: 340 }, params: { text: "oenapu" } },
      { key: "in_rev", descriptorId: "text_input", position: { x: 40, y: 490 }, params: { text: "}omed_" } },
      // Decoders (col 1)
      { key: "d_b64", descriptorId: "base64_decode", position: { x: 300, y: 40 } },
      { key: "d_hex", descriptorId: "hex_decode", position: { x: 300, y: 190 } },
      { key: "d_rot", descriptorId: "rot13", position: { x: 300, y: 340 } },
      { key: "d_rev", descriptorId: "reverse", position: { x: 300, y: 490 } },
      // Merge chain (fan-in)
      { key: "c1", descriptorId: "concat", position: { x: 560, y: 95 } },
      { key: "c2", descriptorId: "concat", position: { x: 820, y: 235 } },
      { key: "c3", descriptorId: "concat", position: { x: 1080, y: 370 } },
      // Fan-out analysis
      { key: "rx", descriptorId: "regex_extract", position: { x: 1360, y: 170 }, params: { preset: "flag" } },
      { key: "out", descriptorId: "text_output", position: { x: 1620, y: 190 } },
      { key: "score", descriptorId: "text_score", position: { x: 1360, y: 370 } },
      { key: "qr", descriptorId: "qr_encode", position: { x: 1360, y: 540 }, params: { scale: 6 } },
      { key: "in_expected", descriptorId: "text_input", position: { x: 1080, y: 620 }, params: { text: "flag{multi_branch_demo}" } },
      { key: "cmp", descriptorId: "compare", position: { x: 1360, y: 720 }, params: { op: "==" } },
    ],
    edges: [
      { from: { node: "in_b64", port: "text" }, to: { node: "d_b64", port: "text" } },
      { from: { node: "in_hex", port: "text" }, to: { node: "d_hex", port: "text" } },
      { from: { node: "in_rot", port: "text" }, to: { node: "d_rot", port: "text" } },
      { from: { node: "in_rev", port: "text" }, to: { node: "d_rev", port: "text" } },
      { from: { node: "d_b64", port: "text" }, to: { node: "c1", port: "a" } },
      { from: { node: "d_hex", port: "text" }, to: { node: "c1", port: "b" } },
      { from: { node: "c1", port: "text" }, to: { node: "c2", port: "a" } },
      { from: { node: "d_rot", port: "text" }, to: { node: "c2", port: "b" } },
      { from: { node: "c2", port: "text" }, to: { node: "c3", port: "a" } },
      { from: { node: "d_rev", port: "text" }, to: { node: "c3", port: "b" } },
      { from: { node: "c3", port: "text" }, to: { node: "rx", port: "text" } },
      { from: { node: "rx", port: "text" }, to: { node: "out", port: "text" } },
      { from: { node: "c3", port: "text" }, to: { node: "score", port: "text" } },
      { from: { node: "c3", port: "text" }, to: { node: "qr", port: "text" } },
      { from: { node: "c3", port: "text" }, to: { node: "cmp", port: "a" } },
      { from: { node: "in_expected", port: "text" }, to: { node: "cmp", port: "b" } },
    ],
  },
  {
    id: "base64-basic",
    name: "Base64 解码",
    description: "最常见的第一步：把 Base64 文本还原为明文。",
    category: "编码解码",
    icon: Binary,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "ZmxhZ3tiYXNlNjR9" } },
      { key: "dec", descriptorId: "base64_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "base-family",
    name: "Base 编码大全对比",
    description:
      "同一段文本同时经 Base32 / Base58 / Base62 / Base85 编码，直观对比各 Base 家族的输出形态。",
    category: "编码解码",
    icon: Binary,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: 40, y: 250 }, params: { text: "flag{base_family}" } },
      { key: "b32", descriptorId: "base32_encode", position: { x: 340, y: 40 } },
      { key: "b58", descriptorId: "base58_encode", position: { x: 340, y: 180 } },
      { key: "b62", descriptorId: "base62_encode", position: { x: 340, y: 320 } },
      { key: "b85", descriptorId: "base85_encode", position: { x: 340, y: 460 } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "b32", port: "data" } },
      { from: { node: "in", port: "text" }, to: { node: "b58", port: "data" } },
      { from: { node: "in", port: "text" }, to: { node: "b62", port: "data" } },
      { from: { node: "in", port: "text" }, to: { node: "b85", port: "data" } },
    ],
  },
  {
    id: "magic-decode",
    name: "万能自动解码",
    description: "自动识别编码并逐层解码，直到出现 flag。拿到一串乱码先试它。",
    category: "编码解码",
    icon: Wand2,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "ZmxhZ3tiYXNlNjR9" } },
      { key: "magic", descriptorId: "magic_decode", position: { x: X(1), y: Y }, params: { pattern: "flag\\{[^}]*\\}", depth: 8 } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "magic", port: "text" } },
      { from: { node: "magic", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "loop-decode",
    name: "循环解码（套娃）",
    description: "对同一种编码重复解码，处理 Base64 套 Base64 这类多层嵌套。",
    category: "编码解码",
    icon: Repeat,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "ZmxhZ3tiYXNlNjR9" } },
      { key: "loop", descriptorId: "loop_decode", position: { x: X(1), y: Y }, params: { codec: "Base64", until: "无法继续", max: 16 } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "loop", port: "text" } },
      { from: { node: "loop", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "xor-brute",
    name: "XOR 单字节爆破",
    description: "单字节密钥未知时，爆破 0-255 并按可读性排序，取最像明文的结果。",
    category: "编码解码",
    icon: Bomb,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "" } },
      { key: "xor", descriptorId: "xor_bruteforce", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "xor", port: "text" } },
      { from: { node: "xor", port: "best" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "hex-decode",
    name: "Hex 解码",
    description: "十六进制字符串转回文本，常与 Base64、XOR 组合出现。",
    category: "编码解码",
    icon: Hash,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "666c61677b6865787d" } },
      { key: "hex", descriptorId: "hex_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "hex", port: "text" } },
      { from: { node: "hex", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "rot13",
    name: "ROT13 / 凯撒",
    description: "字母表轮转 13 位，最经典的替换密码。",
    category: "编码解码",
    icon: RotateCw,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "synt{ebg13}" } },
      { key: "rot", descriptorId: "rot13", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "rot", port: "text" } },
      { from: { node: "rot", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "regex-flag",
    name: "正则提取 Flag",
    description: "从大段日志 / 输出里直接抠出 flag{...}，省去肉眼查找。",
    category: "文本处理",
    icon: Regex,
    nodes: [
      {
        key: "in",
        descriptorId: "text_input",
        position: { x: X(0), y: Y },
        params: { text: "服务器日志里混着一个 flag{regex_found} ，把它揪出来。" },
      },
      { key: "re", descriptorId: "regex_extract", position: { x: X(1), y: Y }, params: { preset: "flag" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "re", port: "text" } },
      { from: { node: "re", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "zero-width-reveal",
    name: "零宽字符隐写还原",
    description:
      "把秘密写进不可见的零宽字符、藏进一句正常的话，再自动侦测符号映射并还原。演示零宽隐写的编码 → 解码闭环。",
    category: "文本处理",
    icon: EyeOff,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "flag{zero_width_secret}" } },
      { key: "enc", descriptorId: "zero_width_encode", position: { x: X(1), y: Y }, params: { cover: "这看起来只是一句普通的话。" } },
      { key: "dec", descriptorId: "zero_width_decode", position: { x: X(2), y: Y }, params: { scheme: "自动" } },
      { key: "out", descriptorId: "text_output", position: { x: X(3), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "enc", port: "text" } },
      { from: { node: "enc", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "hash-compute",
    name: "哈希计算",
    description: "对文本一键算 SHA-256（可切 MD5 / SHA1 / SHA3 / CRC32 等十余种），用于校验或与目标比对。",
    category: "密码学",
    icon: Fingerprint,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "flag{hash_me}" } },
      { key: "h", descriptorId: "hash", position: { x: X(1), y: Y }, params: { algorithm: "SHA256" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "h", port: "data" } },
      { from: { node: "h", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "vigenere-decrypt",
    name: "维吉尼亚解密",
    description: "已知密钥还原维吉尼亚密文（示例：密钥 KEY，RIJVS → HELLO）。改 operation 为加密即可反向。",
    category: "密码学",
    icon: RotateCw,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "RIJVS" } },
      { key: "v", descriptorId: "vigenere", position: { x: X(1), y: Y }, params: { operation: "解密", key: "KEY" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "v", port: "text" } },
      { from: { node: "v", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "aes-decrypt",
    name: "AES-CBC 解密",
    description: "把密文(Hex)、密钥、IV 填入即可解密。支持 CBC/ECB/CTR 与 128/192/256 位密钥。",
    category: "密码学",
    icon: Lock,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "" } },
      {
        key: "a",
        descriptorId: "aes",
        position: { x: X(1), y: Y },
        params: { operation: "解密", mode: "CBC", keyFormat: "Hex", ivFormat: "Hex", inputFormat: "Hex", outputFormat: "UTF8" },
      },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "a", port: "text" } },
      { from: { node: "a", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "foreach-hash",
    name: "批量哈希 (for-each)",
    description: "for 循环生成 1..8，逐项算 SHA-256，再合并成多行——演示 范围 → 逐项映射 → 合并 的数据流循环。",
    category: "控制/流程",
    icon: Network,
    nodes: [
      { key: "r", descriptorId: "range", position: { x: X(0), y: Y }, params: { start: 1, end: 8, step: 1 } },
      { key: "m", descriptorId: "map", position: { x: X(1), y: Y }, params: { op: "SHA256" } },
      { key: "j", descriptorId: "join_list", position: { x: X(2), y: Y }, params: { sep: "换行" } },
      { key: "out", descriptorId: "text_output", position: { x: X(3), y: Y } },
    ],
    edges: [
      { from: { node: "r", port: "list" }, to: { node: "m", port: "list" } },
      { from: { node: "m", port: "list" }, to: { node: "j", port: "list" } },
      { from: { node: "j", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "iterate-decode",
    name: "循环解码 (while)",
    description: "反复应用同一操作，直到命中正则。示例：对套娃 Base64 反复解码，直到出现 flag。",
    category: "控制/流程",
    icon: Repeat,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "ZmxhZ3tpdGVyfQ==" } },
      { key: "it", descriptorId: "iterate", position: { x: X(1), y: Y }, params: { op: "Base64解码", until: "flag\\{[^}]*\\}", max: 16 } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "it", port: "text" } },
      { from: { node: "it", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "qr-decode",
    name: "二维码解码",
    description: "导入二维码 / 条码图片，解析其中隐藏的内容。",
    category: "取证/文件",
    icon: ScanLine,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "qr", descriptorId: "qr_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "qr", port: "image" } },
      { from: { node: "qr", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "archive-extract",
    name: "压缩包解压",
    description: "导入 zip / 7z / rar / gz，自动识别格式并解包读取内容。",
    category: "取证/文件",
    icon: FileArchive,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "ax", descriptorId: "archive_extract", position: { x: X(1), y: Y }, params: { format: "自动" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ax", port: "archive" } },
      { from: { node: "ax", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "qr-encode",
    name: "二维码生成",
    description: "把文本编码成二维码并在节点上直接预览。",
    category: "编码解码",
    icon: QrCode,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "flag{qr_code}" } },
      { key: "qr", descriptorId: "qr_encode", position: { x: X(1), y: Y }, params: { scale: 8 } },
    ],
    edges: [{ from: { node: "in", port: "text" }, to: { node: "qr", port: "text" } }],
  },
];
