// Display order of node categories, shared by the canvas library, the modules
// page and the help index (unlisted categories sort to the end).
export const CATEGORY_ORDER = [
  "输入输出",
  "编码/加密",
  "进制转换",
  "字符编码",
  "加密解密",
  "哈希/摘要",
  "压缩包",
  "隐写术",
  "图像处理",
  "音频处理",
  "二进制分析",
  "可视化分析",
  "文本处理",
  "控制/逻辑",
  "工具/分析",
  "AI",
  "自定义",
];

export function categoryRank(c: string): number {
  const i = CATEGORY_ORDER.indexOf(c);
  return i < 0 ? CATEGORY_ORDER.length : i;
}
