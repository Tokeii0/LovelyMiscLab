# 故事模式立绘 / 场景素材（base64 内联）

这些图在**构建时被 Vite 内联成 base64**（见 `vite.config.ts` 的 `assetsInlineLimit`
只对 `src/assets/galgame/` 生效），所以打包出来的 JS 自包含、不额外附带图片文件。
引用集中在 [`index.ts`](./index.ts)（`SPRITES` / `BACKGROUNDS`）。

> 换图：替换本目录同名文件即可（`pnpm dev` 下热更新即见；生产 `pnpm build` 时内联）。
> 加/减变体或改「情绪↔背景」映射：改 `index.ts` 里的数组 / 映射。

## 角色立绘（每种情绪 2 张，随机切换）

每回合按种子随机挑一张变体。透明底 PNG，源图 1024×1536，程序统一裁到 660×1492；
同情绪两张保持同样的人物大小/镜头/居中，切换才不跳。

| 情绪 | 变体 A | 变体 B |
|---|---|---|
| 平静 | `misca-neutral.png` | `misca-neutral-2.png` |
| 微笑 | `misca-happy.png` | `misca-happy-2.png` |
| 思考 | `misca-thinking.png` | `misca-thinking-2.png` |
| 担心 | `misca-worried.png` | `misca-worried-2.png` |
| 兴奋 | `misca-excited.png` | `misca-excited-2.png` |

## 场景背景（按情绪切换）

16:9，`object-cover` 铺满并叠暗色遮罩。当前映射：平静→`bg-2` / 微笑→`bg-1` /
思考→`bg-3` / 担心→`bg-4` / 兴奋→`bg-5`。JPG/PNG/WebP 均可（无需透明）。

> 提示：内联会让 JS 体积增加约图片总量的 ~1.37 倍（当前主包约 10.7 MB）。想更轻可把
> 立绘转成 WebP（带透明、体积约为 PNG 的 1/3）再放进来。
