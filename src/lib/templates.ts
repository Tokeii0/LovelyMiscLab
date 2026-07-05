import {
  Activity,
  BarChart3,
  Binary,
  Bomb,
  Camera,
  EyeOff,
  FileArchive,
  FileSearch,
  Fingerprint,
  Hash,
  ImageDown,
  KeyRound,
  Layers,
  type LucideIcon,
  Network,
  QrCode,
  Lock,
  Radio,
  Regex,
  Repeat,
  RotateCw,
  ScanLine,
  Shuffle,
  Wand2,
  Wrench,
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

export const TEMPLATE_CATEGORIES = [
  "综合演示",
  "编码解码",
  "文本处理",
  "密码学",
  "控制/流程",
  "隐写术",
  "取证/文件",
  "二进制分析",
  "可视化分析",
  "音频处理",
  "出题",
] as const;

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
    category: "隐写术",
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
    id: "imagein-extract",
    name: "imageIN 图片取文件",
    description:
      "把 imageIN（图影）隐写图片里的文件还原出来：导入图片 → imageIN 文件提取（自动识别深度与排布、GBK 文件名）→ 一路识别文件类型、一路导出文件，并显示深度/文件名/大小。真实文件名见提取节点的『文件名』输出。",
    category: "隐写术",
    icon: ImageDown,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 250 } },
      { key: "ex", descriptorId: "imagein_extract", position: { x: 320, y: 250 } },
      { key: "ft", descriptorId: "detect_file_type", position: { x: 640, y: 90 } },
      { key: "save", descriptorId: "file_output", position: { x: 640, y: 250 }, params: { filename: "提取文件.bin" } },
      { key: "info", descriptorId: "text_output", position: { x: 640, y: 410 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ex", port: "data" } },
      { from: { node: "ex", port: "bytes" }, to: { node: "ft", port: "data" } },
      { from: { node: "ex", port: "bytes" }, to: { node: "save", port: "data" } },
      { from: { node: "ex", port: "report" }, to: { node: "info", port: "text" } },
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

  // ---------------------------------------------------------------- 编码解码
  {
    id: "base32-decode",
    name: "Base32 解码",
    description: "Base32 编码还原为明文，常见于第二梯队编码。",
    category: "编码解码",
    icon: Binary,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "MZWGCZ33MJQXGZJTGJPW6235" } },
      { key: "dec", descriptorId: "base32_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "base58-decode",
    name: "Base58 解码",
    description: "比特币/短链常用的 Base58，去掉了易混字符。",
    category: "编码解码",
    icon: Binary,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "3sCWBxPb32JKGDDB3y1dv" } },
      { key: "dec", descriptorId: "base58_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "morse-decode",
    name: "摩尔斯解码",
    description: "点划电码转回文本，字母间空格、单词间 /。",
    category: "编码解码",
    icon: Radio,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "..-. .-.. .- --. / .... . .-.. .-.. ---" } },
      { key: "dec", descriptorId: "morse_decode", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "binary-decode",
    name: "二进制转文本",
    description: "8 位一组的 0/1 串还原为 ASCII 文本。",
    category: "编码解码",
    icon: Binary,
    nodes: [
      {
        key: "in",
        descriptorId: "text_input",
        position: { x: X(0), y: Y },
        params: { text: "01100110 01101100 01100001 01100111 01111011 01100010 01101001 01101110 01100001 01110010 01111001 01111101" },
      },
      { key: "dec", descriptorId: "from_binary", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "charcode-decode",
    name: "码点转字符",
    description: "空格分隔的十六进制码点转回字符（可切 10/8/2 进制）。",
    category: "编码解码",
    icon: Hash,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "66 6c 61 67 7b 63 68 61 72 63 6f 64 65 7d" } },
      { key: "dec", descriptorId: "from_charcode", position: { x: X(1), y: Y }, params: { base: "16", delimiter: "空格" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "dec", port: "text" } },
      { from: { node: "dec", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },

  // ---------------------------------------------------------------- 密码学
  {
    id: "caesar-cipher",
    name: "凯撒密码",
    description: "字母表整体位移。示例位移 23（=解 +3 加密），改 amount 即可试其它位移。",
    category: "密码学",
    icon: RotateCw,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "iodj{fdhvdu}" } },
      { key: "c", descriptorId: "caesar", position: { x: X(1), y: Y }, params: { amount: 23 } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "c", port: "text" } },
      { from: { node: "c", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "atbash-cipher",
    name: "Atbash 密码",
    description: "字母表反射（a↔z）。自反，编码解码同一操作。",
    category: "密码学",
    icon: Shuffle,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "uozt{zgyzhs}" } },
      { key: "a", descriptorId: "atbash", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "a", port: "text" } },
      { from: { node: "a", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "railfence-decode",
    name: "栅栏密码解密",
    description: "W 型栅栏（zigzag）转置还原。示例 3 栏，改 rails 试其它栏数。",
    category: "密码学",
    icon: Shuffle,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "f{lnlgri_ec}aafe" } },
      { key: "rf", descriptorId: "rail_fence_decode", position: { x: X(1), y: Y }, params: { rails: 3 } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "rf", port: "text" } },
      { from: { node: "rf", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "rc4-decrypt",
    name: "RC4 解密",
    description: "填入密文(Hex)与密钥即可解。RC4 加解密对称，改 operation/格式即可反向。",
    category: "密码学",
    icon: Lock,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "" } },
      {
        key: "rc4",
        descriptorId: "rc4",
        position: { x: X(1), y: Y },
        params: { key: "", keyFormat: "UTF8", inputFormat: "Hex", outputFormat: "UTF8" },
      },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "rc4", port: "text" } },
      { from: { node: "rc4", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "rsa-recover-d",
    name: "RSA 求私钥 d",
    description: "已知素数 p、q 与公钥指数 e，算出 n、φ(n) 与私钥 d —— CTF 里最常见的 RSA 起手。",
    category: "密码学",
    icon: KeyRound,
    nodes: [
      { key: "rsa", descriptorId: "rsa_params", position: { x: X(0), y: Y }, params: { p: "61", q: "53", e: "17" } },
      { key: "out", descriptorId: "text_output", position: { x: X(1), y: Y } },
    ],
    edges: [{ from: { node: "rsa", port: "text" }, to: { node: "out", port: "text" } }],
  },

  // ---------------------------------------------------------------- 文本处理
  {
    id: "char-frequency",
    name: "字符频率统计",
    description: "统计各字符出现次数，替换密码/词频分析的起点。",
    category: "文本处理",
    icon: BarChart3,
    nodes: [
      {
        key: "in",
        descriptorId: "text_input",
        position: { x: X(0), y: Y },
        params: { text: "the quick brown fox jumps over the lazy dog the end" },
      },
      { key: "cf", descriptorId: "char_frequency", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "cf", port: "text" } },
      { from: { node: "cf", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },

  // ---------------------------------------------------------------- 隐写术
  {
    id: "image-stego-triage",
    name: "图片隐写一键尝试",
    description:
      "导入一张图片后并行跑一遍常见图片隐写检查：识别真实类型/后缀（识破改错的扩展名或藏在图里的其它文件）、读 EXIF 与注释、提取 LSB 载荷、抽位平面肉眼看隐藏图案，并在原始字节里正则搜 flag。拿到图片先跑这个。",
    category: "隐写术",
    icon: Wand2,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 340 } },
      { key: "ft", descriptorId: "detect_file_type", position: { x: 340, y: 40 } },
      { key: "ftout", descriptorId: "text_output", position: { x: 640, y: 40 } },
      { key: "exif", descriptorId: "exif_extract", position: { x: 340, y: 190 } },
      { key: "exifout", descriptorId: "text_output", position: { x: 640, y: 190 } },
      { key: "lsb", descriptorId: "lsb_extract", position: { x: 340, y: 340 }, params: { channels: "RGB", bit: 0 } },
      { key: "lsbout", descriptorId: "text_output", position: { x: 640, y: 340 } },
      { key: "dec", descriptorId: "decode_text", position: { x: 340, y: 490 }, params: { charset: "ISO-8859-1" } },
      { key: "ext", descriptorId: "extract", position: { x: 640, y: 490 }, params: { kind: "flag", unique: true } },
      { key: "extout", descriptorId: "text_output", position: { x: 940, y: 490 } },
      { key: "bp", descriptorId: "bit_plane", position: { x: 340, y: 640 }, params: { channel: "R", bit: 0 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ft", port: "data" } },
      { from: { node: "ft", port: "text" }, to: { node: "ftout", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "exif", port: "data" } },
      { from: { node: "exif", port: "text" }, to: { node: "exifout", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "lsb", port: "data" } },
      { from: { node: "lsb", port: "text" }, to: { node: "lsbout", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "dec", port: "data" } },
      { from: { node: "dec", port: "text" }, to: { node: "ext", port: "text" } },
      { from: { node: "ext", port: "text" }, to: { node: "extout", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "bp", port: "data" } },
    ],
  },
  {
    id: "lsb-extract",
    name: "LSB 位隐写提取",
    description: "导入 PNG/BMP，按位平面读取 RGB 最低位拼回隐藏数据。图片隐写第一梯队。",
    category: "隐写术",
    icon: ImageDown,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "lsb", descriptorId: "lsb_extract", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "lsb", port: "data" } },
      { from: { node: "lsb", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "stegcloak-reveal",
    name: "StegCloak 解码",
    description: "从掺入零宽字符的文本里取回秘密（可带密码）。把载体文本粘进输入即可。",
    category: "隐写术",
    icon: EyeOff,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "" } },
      { key: "sc", descriptorId: "stegcloak_reveal", position: { x: X(1), y: Y }, params: { password: "" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "sc", port: "text" } },
      { from: { node: "sc", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "cloacked-pixel-extract",
    name: "cloacked-pixel 提取",
    description: "导入图片 + 密码，解出 AES 加密后藏在 LSB 里的载荷。",
    category: "隐写术",
    icon: Lock,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "cp", descriptorId: "cloacked_pixel_extract", position: { x: X(1), y: Y }, params: { password: "" } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "cp", port: "data" } },
      { from: { node: "cp", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "bit-plane",
    name: "位平面提取",
    description: "抽出某通道的单个位平面成黑白图，肉眼找隐藏图案/二维码。",
    category: "隐写术",
    icon: Layers,
    nodes: [
      { key: "img", descriptorId: "image_input", position: { x: X(0), y: Y } },
      { key: "bp", descriptorId: "bit_plane", position: { x: X(1), y: Y }, params: { channel: "R", bit: 0 } },
    ],
    edges: [{ from: { node: "img", port: "bytes" }, to: { node: "bp", port: "data" } }],
  },

  // ---------------------------------------------------------------- 取证/文件
  {
    id: "filetype-detect",
    name: "文件类型识别",
    description: "读文件魔数判断真实类型，识破改错的后缀名。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "ft", descriptorId: "detect_file_type", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ft", port: "data" } },
      { from: { node: "ft", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "exif-view",
    name: "EXIF 信息",
    description: "读出照片的 EXIF 元数据（拍摄时间、GPS、相机等）。",
    category: "取证/文件",
    icon: Camera,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "ex", descriptorId: "exif_extract", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ex", port: "data" } },
      { from: { node: "ex", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "png-chunks",
    name: "PNG Chunk 检查",
    description: "列出 PNG 的 IHDR / IDAT / IEND / tEXt 等 chunk，检查长度、属性和 CRC，定位注释、私有块和尾部附加数据。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "png", descriptorId: "png_chunks", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "png", port: "data" } },
      { from: { node: "png", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "jpeg-markers",
    name: "JPEG Marker 检查",
    description: "列出 JPEG 的 SOI、APPn、COM、DQT、SOS、EOI 等 marker，快速查看注释段、EXIF 段和 EOI 后附加数据。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "jpg", descriptorId: "jpeg_markers", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "jpg", port: "data" } },
      { from: { node: "jpg", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "gif-blocks",
    name: "GIF Block 检查",
    description: "解析 GIF Header、扩展块、图像块、子块和 Trailer，查看 Comment/Application Extension 等常见藏点。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "gif", descriptorId: "gif_blocks", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "gif", port: "data" } },
      { from: { node: "gif", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "zip-directory-diff",
    name: "ZIP 目录差异检查",
    description: "比较 ZIP 本地文件头和中央目录中的 flags、CRC、大小、压缩方式等字段，定位伪加密和目录篡改。",
    category: "取证/文件",
    icon: FileArchive,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "diff", descriptorId: "zip_directory_diff", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "diff", port: "archive" } },
      { from: { node: "diff", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "pdf-objects",
    name: "PDF Object 检查",
    description: "列出 PDF indirect object，查看 Type/Subtype/Filter、stream 位置和对象预览。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "pdf", descriptorId: "pdf_objects", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "pdf", port: "data" } },
      { from: { node: "pdf", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "pdf-streams",
    name: "PDF Stream 检查",
    description: "枚举 PDF stream，默认尝试 FlateDecode，首个 stream 字节可继续接文件识别或导出。",
    category: "取证/文件",
    icon: FileSearch,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "pdf", descriptorId: "pdf_streams", position: { x: X(1), y: Y }, params: { decodeFlate: true } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "pdf", port: "data" } },
      { from: { node: "pdf", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "ooxml-metadata",
    name: "OOXML 元数据检查",
    description: "读取 docx/xlsx/pptx 的 docProps/core.xml、app.xml 和 custom.xml，提取标题、作者、应用、自定义属性等。",
    category: "取证/文件",
    icon: FileArchive,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "meta", descriptorId: "ooxml_metadata", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "meta", port: "document" } },
      { from: { node: "meta", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "ooxml-embedded",
    name: "OOXML 内嵌资源检查",
    description: "列出 Office 文档中的 embeddings、media、ActiveX、OLE 和 vbaProject.bin，首个资源可继续导出或识别类型。",
    category: "取证/文件",
    icon: FileArchive,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "emb", descriptorId: "ooxml_embedded", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "emb", port: "document" } },
      { from: { node: "emb", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "apk-manifest",
    name: "APK Manifest 检查",
    description: "读取 APK 的 AndroidManifest.xml，解析 binary XML 字符串池、包名、权限和组件。",
    category: "取证/文件",
    icon: FileArchive,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "manifest", descriptorId: "apk_manifest", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "manifest", port: "apk" } },
      { from: { node: "manifest", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "dex-strings",
    name: "DEX 字符串检查",
    description: "按 DEX string_ids 表提取字符串，适合从 APK classes.dex 或单独 DEX 中快速找包名、URL、flag 和敏感字面量。",
    category: "取证/文件",
    icon: Binary,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "dex", descriptorId: "dex_strings", position: { x: X(1), y: Y }, params: { minLen: 1, maxCount: 2000, showOffset: true } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "dex", port: "dex" } },
      { from: { node: "dex", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "entropy-check",
    name: "香农熵分析",
    description: "算文件的字节熵，判断是否加密/压缩（高熵≈随机）或藏了东西。",
    category: "取证/文件",
    icon: Activity,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "en", descriptorId: "entropy", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "en", port: "data" } },
      { from: { node: "en", port: "text" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "png-fix",
    name: "PNG 宽高修复",
    description: "PNG 被改了 IHDR 宽高导致显示不全时，CRC 爆破还原正确尺寸。",
    category: "取证/文件",
    icon: Wrench,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: X(0), y: Y } },
      { key: "fix", descriptorId: "png_fix", position: { x: X(1), y: Y }, params: { mode: "CRC 爆破" } },
    ],
    edges: [{ from: { node: "file", port: "bytes" }, to: { node: "fix", port: "data" } }],
  },
  {
    id: "archive-crack",
    name: "压缩包密码爆破",
    description:
      "用字典爆破加密压缩包：导入压缩包 + 口令字典 → 通用口令爆破（目标=解压，判据「无报错」，错口令会解压失败）→ 得到密码与解出的内容。字典想用大字典文件时，把文本输入换成「文件导入」的文本输出即可。",
    category: "取证/文件",
    icon: Bomb,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 110 } },
      {
        key: "dict",
        descriptorId: "text_input",
        position: { x: 40, y: 300 },
        params: { text: "123456\npassword\nadmin\nletmein\nqwerty\niloveyou\nflag\nctf\ninfected\n7z" },
      },
      {
        key: "crack",
        descriptorId: "password_crack",
        position: { x: 340, y: 200 },
        params: { node: "archive_extract", passwordParam: "password", success: "无报错(能解出)" },
      },
      { key: "pw", descriptorId: "text_output", position: { x: 660, y: 110 } },
      { key: "content", descriptorId: "text_output", position: { x: 660, y: 300 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "crack", port: "data" } },
      { from: { node: "dict", port: "text" }, to: { node: "crack", port: "wordlist" } },
      { from: { node: "crack", port: "password" }, to: { node: "pw", port: "text" } },
      { from: { node: "crack", port: "text" }, to: { node: "content", port: "text" } },
    ],
  },
  {
    id: "bkcrack-known-plaintext",
    name: "bkcrack 已知明文攻击",
    description:
      "对 ZipCrypto 传统加密的 ZIP 做已知明文攻击（Biham–Kocher，内置原生实现，无需外部程序）：导入加密 ZIP → bkcrack 选「PNG 图片」等明文模板（或自定义 Hex）求出三个内部密钥并解密目标条目 → 一路导出明文文件、一路显示密钥。至少需 12 字节连续已知明文（文件头模板适用于 Stored 条目）。",
    category: "取证/文件",
    icon: KeyRound,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 150 } },
      {
        key: "bk",
        descriptorId: "bkcrack",
        position: { x: 340, y: 150 },
        params: { template: "PNG 图片", decrypt: true },
      },
      { key: "save", descriptorId: "file_output", position: { x: 660, y: 60 }, params: { filename: "解密结果.bin" } },
      { key: "keys", descriptorId: "text_output", position: { x: 660, y: 240 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "bk", port: "archive" } },
      { from: { node: "bk", port: "data" }, to: { node: "save", port: "data" } },
      { from: { node: "bk", port: "keys" }, to: { node: "keys", port: "text" } },
    ],
  },
  {
    id: "binary-triage",
    name: "可执行文件分析起手",
    description:
      "导入 ELF / PE / Mach-O 后并行查看文件头、节区、导入导出、字符串与 overlay；适合逆向/取证题的第一轮结构检查。",
    category: "二进制分析",
    icon: Binary,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 360 } },
      { key: "info", descriptorId: "binary_info", position: { x: 340, y: 40 } },
      { key: "info_out", descriptorId: "text_output", position: { x: 660, y: 40 } },
      { key: "sections", descriptorId: "binary_sections", position: { x: 340, y: 190 } },
      { key: "sections_out", descriptorId: "text_output", position: { x: 660, y: 190 } },
      { key: "symbols", descriptorId: "binary_symbols", position: { x: 340, y: 340 }, params: { kind: "导入" } },
      { key: "symbols_out", descriptorId: "text_output", position: { x: 660, y: 340 } },
      { key: "str", descriptorId: "strings", position: { x: 340, y: 490 }, params: { encoding: "两者", showOffset: true } },
      { key: "str_out", descriptorId: "text_output", position: { x: 660, y: 490 } },
      { key: "overlay", descriptorId: "binary_overlay", position: { x: 340, y: 660 } },
      { key: "otype", descriptorId: "detect_file_type", position: { x: 660, y: 640 } },
      { key: "otype_out", descriptorId: "text_output", position: { x: 960, y: 640 } },
      { key: "osave", descriptorId: "file_output", position: { x: 660, y: 800 }, params: { filename: "overlay.bin" } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "info", port: "data" } },
      { from: { node: "info", port: "text" }, to: { node: "info_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "sections", port: "data" } },
      { from: { node: "sections", port: "text" }, to: { node: "sections_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "symbols", port: "data" } },
      { from: { node: "symbols", port: "text" }, to: { node: "symbols_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "str", port: "data" } },
      { from: { node: "str", port: "text" }, to: { node: "str_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "overlay", port: "data" } },
      { from: { node: "overlay", port: "bytes" }, to: { node: "otype", port: "data" } },
      { from: { node: "otype", port: "text" }, to: { node: "otype_out", port: "text" } },
      { from: { node: "overlay", port: "bytes" }, to: { node: "osave", port: "data" } },
    ],
  },
  {
    id: "pe-deep-triage",
    name: "PE 深度结构检查",
    description:
      "导入 PE 后并行查看资源表、签名证书、imphash、节区熵、壳特征提示和 .NET 元数据，用于恶意样本/逆向题第二轮结构检查。",
    category: "二进制分析",
    icon: Fingerprint,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 430 } },
      { key: "res", descriptorId: "pe_resources", position: { x: 340, y: 40 } },
      { key: "res_out", descriptorId: "text_output", position: { x: 660, y: 40 } },
      { key: "cert", descriptorId: "pe_certificates", position: { x: 340, y: 190 } },
      { key: "cert_out", descriptorId: "text_output", position: { x: 660, y: 190 } },
      { key: "imphash", descriptorId: "pe_imphash", position: { x: 340, y: 340 } },
      { key: "imphash_out", descriptorId: "text_output", position: { x: 660, y: 340 } },
      { key: "entropy", descriptorId: "section_entropy", position: { x: 340, y: 490 } },
      { key: "entropy_out", descriptorId: "text_output", position: { x: 660, y: 490 } },
      { key: "packer", descriptorId: "pe_packer_hints", position: { x: 340, y: 640 } },
      { key: "packer_out", descriptorId: "text_output", position: { x: 660, y: 640 } },
      { key: "dotnet", descriptorId: "dotnet_metadata", position: { x: 340, y: 790 } },
      { key: "dotnet_out", descriptorId: "text_output", position: { x: 660, y: 790 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "res", port: "data" } },
      { from: { node: "res", port: "text" }, to: { node: "res_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "cert", port: "data" } },
      { from: { node: "cert", port: "text" }, to: { node: "cert_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "imphash", port: "data" } },
      { from: { node: "imphash", port: "hash" }, to: { node: "imphash_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "entropy", port: "data" } },
      { from: { node: "entropy", port: "text" }, to: { node: "entropy_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "packer", port: "data" } },
      { from: { node: "packer", port: "text" }, to: { node: "packer_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "dotnet", port: "data" } },
      { from: { node: "dotnet", port: "text" }, to: { node: "dotnet_out", port: "text" } },
    ],
  },
  {
    id: "byte-visual-triage",
    name: "字节可视化分析",
    description:
      "导入任意文件后同时绘制字节直方图、熵曲线和字节分布图，快速判断文本区、压缩/加密区、填充区和内嵌数据边界。",
    category: "可视化分析",
    icon: BarChart3,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 260 } },
      { key: "hist", descriptorId: "byte_histogram", position: { x: 340, y: 60 } },
      { key: "entropy", descriptorId: "entropy_curve", position: { x: 340, y: 260 }, params: { window: 512 } },
      { key: "entropy_out", descriptorId: "text_output", position: { x: 660, y: 260 } },
      { key: "map", descriptorId: "byte_map", position: { x: 340, y: 460 }, params: { width: 256 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "hist", port: "data" } },
      { from: { node: "file", port: "bytes" }, to: { node: "entropy", port: "data" } },
      { from: { node: "entropy", port: "text" }, to: { node: "entropy_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "map", port: "data" } },
    ],
  },
  {
    id: "audio-stego-triage",
    name: "音频隐写起手检查",
    description:
      "导入 WAV 后并行查看基础信息、频谱图、WAV LSB 和 DTMF 解码结果；适合处理声音频域藏字、采样低位藏数据和拨号音题。",
    category: "音频处理",
    icon: Radio,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 300 } },
      { key: "info", descriptorId: "audio_info", position: { x: 340, y: 40 } },
      { key: "info_out", descriptorId: "text_output", position: { x: 660, y: 40 } },
      { key: "spec", descriptorId: "audio_spectrogram", position: { x: 340, y: 210 }, params: { fftSize: "1024", overlap: "75%" } },
      { key: "lsb", descriptorId: "wav_lsb", position: { x: 340, y: 400 } },
      { key: "lsb_out", descriptorId: "text_output", position: { x: 660, y: 400 } },
      { key: "dtmf", descriptorId: "dtmf_decode", position: { x: 340, y: 560 } },
      { key: "dtmf_out", descriptorId: "text_output", position: { x: 660, y: 560 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "info", port: "data" } },
      { from: { node: "info", port: "text" }, to: { node: "info_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "spec", port: "data" } },
      { from: { node: "file", port: "bytes" }, to: { node: "lsb", port: "data" } },
      { from: { node: "lsb", port: "text" }, to: { node: "lsb_out", port: "text" } },
      { from: { node: "file", port: "bytes" }, to: { node: "dtmf", port: "data" } },
      { from: { node: "dtmf", port: "text" }, to: { node: "dtmf_out", port: "text" } },
    ],
  },
  {
    id: "deepsound-extract",
    name: "DeepSound 隐藏文件提取",
    description:
      "导入 DeepSound 生成的 WAV，填写密码（如有），提取首个隐藏文件并保存，同时查看文件名和分析报告。",
    category: "音频处理",
    icon: KeyRound,
    nodes: [
      { key: "file", descriptorId: "file_import", position: { x: 40, y: 180 } },
      { key: "ds", descriptorId: "deepsound_extract", position: { x: 340, y: 180 }, params: { password: "" } },
      { key: "save", descriptorId: "file_output", position: { x: 660, y: 80 }, params: { filename: "deepsound.bin" } },
      { key: "report", descriptorId: "text_output", position: { x: 660, y: 260 } },
    ],
    edges: [
      { from: { node: "file", port: "bytes" }, to: { node: "ds", port: "data" } },
      { from: { node: "ds", port: "bytes" }, to: { node: "save", port: "data" } },
      { from: { node: "ds", port: "report" }, to: { node: "report", port: "text" } },
    ],
  },
  {
    id: "zip-pseudo-encrypt-challenge",
    name: "ZIP 伪加密出题",
    description:
      "把 flag 文本打包成 ZIP，再置伪加密位并保存。生成的文件会提示需要密码，但可用伪加密修复还原。",
    category: "出题",
    icon: FileArchive,
    nodes: [
      { key: "flag", descriptorId: "text_input", position: { x: 40, y: 180 }, params: { text: "flag{fake_zip_encryption}" } },
      { key: "zip", descriptorId: "zip_create", position: { x: 340, y: 180 }, params: { filename: "flag.txt", method: "Stored" } },
      { key: "fake", descriptorId: "zip_pseudo_encrypt", position: { x: 640, y: 180 } },
      { key: "save", descriptorId: "file_output", position: { x: 940, y: 80 }, params: { filename: "challenge.zip" } },
      { key: "report", descriptorId: "text_output", position: { x: 940, y: 260 } },
    ],
    edges: [
      { from: { node: "flag", port: "text" }, to: { node: "zip", port: "data" } },
      { from: { node: "zip", port: "bytes" }, to: { node: "fake", port: "data" } },
      { from: { node: "fake", port: "bytes" }, to: { node: "save", port: "data" } },
      { from: { node: "fake", port: "report" }, to: { node: "report", port: "text" } },
    ],
  },
  {
    id: "lsb-embed-challenge",
    name: "LSB 图片隐写出题",
    description:
      "选择一张封面图，把载荷文本写进 RGB 最低位，生成隐写 PNG；同时接回 LSB 提取做自检。",
    category: "出题",
    icon: ImageDown,
    nodes: [
      { key: "cover", descriptorId: "image_input", position: { x: 40, y: 80 } },
      { key: "payload", descriptorId: "text_input", position: { x: 40, y: 300 }, params: { text: "flag{lsb_payload}" } },
      { key: "embed", descriptorId: "lsb_embed", position: { x: 340, y: 180 }, params: { channels: "RGB", bit: 0, msbFirst: true } },
      { key: "view", descriptorId: "image_view", position: { x: 660, y: 80 } },
      { key: "save", descriptorId: "file_output", position: { x: 660, y: 260 }, params: { filename: "stego.png" } },
      { key: "check", descriptorId: "lsb_extract", position: { x: 660, y: 440 }, params: { channels: "RGB", bit: 0, msbFirst: true } },
      { key: "check_out", descriptorId: "text_output", position: { x: 960, y: 440 } },
    ],
    edges: [
      { from: { node: "cover", port: "bytes" }, to: { node: "embed", port: "cover" } },
      { from: { node: "payload", port: "text" }, to: { node: "embed", port: "payload" } },
      { from: { node: "embed", port: "image" }, to: { node: "view", port: "image" } },
      { from: { node: "embed", port: "bytes" }, to: { node: "save", port: "data" } },
      { from: { node: "embed", port: "bytes" }, to: { node: "check", port: "data" } },
      { from: { node: "check", port: "text" }, to: { node: "check_out", port: "text" } },
    ],
  },
  {
    id: "caesar-bruteforce",
    name: "凯撒爆破",
    description: "不知道位移时直接试完 25 种凯撒偏移，取评分最高的候选文本。",
    category: "密码学",
    icon: Bomb,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "iodj{fdhvdu_euxwh}" } },
      { key: "brute", descriptorId: "caesar_brute", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "brute", port: "text" } },
      { from: { node: "brute", port: "best" }, to: { node: "out", port: "text" } },
    ],
  },
  {
    id: "vigenere-auto-analysis",
    name: "维吉尼亚自动分析",
    description:
      "不知道密钥时估计维吉尼亚密钥长度，输出候选 key、明文和评分；适合先看候选摘要再取 best。",
    category: "密码学",
    icon: KeyRound,
    nodes: [
      {
        key: "in",
        descriptorId: "text_input",
        position: { x: X(0), y: Y },
        params: { text: "elq ehtgw pezaz tbi ngacd shse elq znkc pct lrp hup jxot tw rznr{zuuryids_nexmqx}" },
      },
      { key: "vig", descriptorId: "vigenere_analyze", position: { x: X(1), y: Y }, params: { maxKeyLen: 12, top: 8 } },
      { key: "best", descriptorId: "text_output", position: { x: X(2), y: Y - 80 } },
      { key: "candidates", descriptorId: "text_output", position: { x: X(2), y: Y + 100 } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "vig", port: "text" } },
      { from: { node: "vig", port: "best" }, to: { node: "best", port: "text" } },
      { from: { node: "vig", port: "text" }, to: { node: "candidates", port: "text" } },
    ],
  },
  {
    id: "repeating-xor-crack",
    name: "重复密钥 XOR 破解",
    description:
      "输入 Hex/Base64/原始密文，自动枚举重复 XOR 密钥长度，输出最可能明文、key 和候选摘要。",
    category: "密码学",
    icon: Bomb,
    nodes: [
      {
        key: "in",
        descriptorId: "text_input",
        position: { x: X(0), y: Y },
        params: {
          text: "2f2f242e38372c332028372c27241a312c371622313d2226223e653d2b2c3a632c3a6324692f2a2724203b632027242920302d69302027372027202069252a3b63362a2c37202d22",
        },
      },
      { key: "xor", descriptorId: "repeating_xor_crack", position: { x: X(1), y: Y }, params: { inputFormat: "Hex", maxKeyLen: 8, top: 8 } },
      { key: "best", descriptorId: "text_output", position: { x: X(2), y: Y - 80 } },
      { key: "key", descriptorId: "text_output", position: { x: X(2), y: Y + 100 } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "xor", port: "data" } },
      { from: { node: "xor", port: "best" }, to: { node: "best", port: "text" } },
      { from: { node: "xor", port: "key" }, to: { node: "key", port: "text" } },
    ],
  },
  {
    id: "classical-bruteforce-pack",
    name: "古典密码爆破组合",
    description:
      "同屏尝试栅栏栏数、仿射 a/b 参数和小列数列换位，适合先不知道古典密码具体类型时快速筛候选。",
    category: "密码学",
    icon: Shuffle,
    nodes: [
      { key: "rail_in", descriptorId: "text_input", position: { x: 40, y: 70 }, params: { text: "f{s_tracra l tlgcascbueoc_tak edbeegihtxalirfet}alnse" } },
      { key: "rail", descriptorId: "rail_fence_bruteforce", position: { x: 340, y: 70 }, params: { maxRails: 8, top: 5 } },
      { key: "rail_out", descriptorId: "text_output", position: { x: 660, y: 70 } },
      { key: "affine_in", descriptorId: "text_input", position: { x: 40, y: 280 }, params: { text: "hlim{sliuuws_npezchapsc_izzisg} pcixinlc cvmlwur zctz" } },
      { key: "affine", descriptorId: "affine_bruteforce", position: { x: 340, y: 280 }, params: { top: 5 } },
      { key: "affine_out", descriptorId: "text_output", position: { x: 660, y: 280 } },
      { key: "column_in", descriptorId: "text_input", position: { x: 40, y: 490 }, params: { text: "acuatnotntc ab gsttfgomrrssi_tkrdlelhel{ln_apioaa}eaeni x" } },
      { key: "column", descriptorId: "columnar_bruteforce", position: { x: 340, y: 490 }, params: { maxColumns: 4, top: 5 } },
      { key: "column_out", descriptorId: "text_output", position: { x: 660, y: 490 } },
    ],
    edges: [
      { from: { node: "rail_in", port: "text" }, to: { node: "rail", port: "text" } },
      { from: { node: "rail", port: "best" }, to: { node: "rail_out", port: "text" } },
      { from: { node: "affine_in", port: "text" }, to: { node: "affine", port: "text" } },
      { from: { node: "affine", port: "best" }, to: { node: "affine_out", port: "text" } },
      { from: { node: "column_in", port: "text" }, to: { node: "column", port: "text" } },
      { from: { node: "column", port: "best" }, to: { node: "column_out", port: "text" } },
    ],
  },
  {
    id: "base64-stego-extract",
    name: "Base64 填充位隐写",
    description:
      "从多行带 = 填充的 Base64 文本里提取被普通解码器忽略的低位隐藏数据。",
    category: "隐写术",
    icon: EyeOff,
    nodes: [
      { key: "in", descriptorId: "text_input", position: { x: X(0), y: Y }, params: { text: "AA==\nAP==" } },
      { key: "stego", descriptorId: "base64_stego", position: { x: X(1), y: Y } },
      { key: "out", descriptorId: "text_output", position: { x: X(2), y: Y } },
      { key: "hex", descriptorId: "text_output", position: { x: X(2), y: Y + 160 } },
    ],
    edges: [
      { from: { node: "in", port: "text" }, to: { node: "stego", port: "text" } },
      { from: { node: "stego", port: "text" }, to: { node: "out", port: "text" } },
      { from: { node: "stego", port: "hex" }, to: { node: "hex", port: "text" } },
    ],
  },
];
