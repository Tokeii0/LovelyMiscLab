# 故事模式立绘 / 场景素材

图片放在 [`public/galgame/`](../../../public/galgame/)，作为普通静态文件随 `dist/galgame/`
一起打包（Vite 原样复制 `public/`），只在打开故事模式时才加载，不会内联进单文件
`index.html`。引用集中在本目录的 [`index.ts`](./index.ts)（`SPRITES` / `BACKGROUNDS`）。

> 换图：替换 `public/galgame/` 下的同名文件即可（`pnpm dev` 下刷新即见）。
> 加/减变体或改「情绪↔背景」映射：改 `index.ts` 里的数组 / 映射。

## 角色立绘（每种情绪 2 张，随机切换）

每回合按种子随机挑一张变体。透明底 WebP，统一裁到 660×1492；同情绪两张保持同样的
人物大小/镜头/居中，切换才不跳。

| 情绪 | 变体 A | 变体 B |
|---|---|---|
| 平静 | `misca-neutral.webp` | `misca-neutral-2.webp` |
| 微笑 | `misca-happy.webp` | `misca-happy-2.webp` |
| 思考 | `misca-thinking.webp` | `misca-thinking-2.webp` |
| 担心 | `misca-worried.webp` | `misca-worried-2.webp` |
| 兴奋 | `misca-excited.webp` | `misca-excited-2.webp` |

## 场景背景（按情绪切换）

16:9，`object-cover` 铺满并叠暗色遮罩。当前映射：平静→`bg-2` / 微笑→`bg-1` /
思考→`bg-3` / 担心→`bg-4` / 兴奋→`bg-5`。
