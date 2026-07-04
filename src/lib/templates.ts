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

export const TEMPLATE_CATEGORIES = ["综合演示", "编码解码", "文本处理", "密码学", "控制/流程", "隐写术", "取证/文件"] as const;

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
];
