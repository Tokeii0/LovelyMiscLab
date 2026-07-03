# 更新日志

本项目所有值得注意的变更都会记录在此文件。版本遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.2.3] - 2026-07-03

### 新增

- **软件更新**：启动时自动检查 GitHub 最新 Release，一键下载并就地替换、自动重启；「设置 → 软件更新」也可手动检查。
- **图片查看器**：节点 / 检查器 / 运行记录里的图片，点击弹出大图。支持缩放、拖动平移、旋转 90°、水平/垂直翻转、中心对称，以及亮度 / 对比度 / 曝光 / 饱和度 / 锐度等 PS 风格实时调整（仅用于观察，不改原图）。
- **JPG 宽高修复**（新节点）：基线 JPEG 通过统计 MCU 数自动恢复被篡改的真实高度（JPEG 无 CRC 可校验），也支持手动指定尺寸。
- **音频处理**（新分类，5 个节点）：
  - 音频信息 —— 解析 WAV 采样率 / 声道 / 位深 / 时长；
  - 音频频谱图 —— STFT 频谱图，显现藏在频域里的文字或图案；
  - WAV LSB 提取 —— 提取采样最低有效位里的隐藏数据；
  - DTMF 拨号音解码 —— Goertzel 检测电话拨号音还原为数字；
  - **DeepSound 提取** —— 还原 DeepSound 藏在 WAV 里的隐藏文件，支持 AES-256 密码（DSC2/DSCF 格式，逆向自官方 DLL 并与真实文件对拍）。

### 改进

- **文件类型识别**：新增「后缀名」输出，方便拼接文件名等用途。
- **PNG 宽高修复**：新增「自动」模式 —— 解压 IDAT 数据流推断真实尺寸，可修复连 CRC 都被改写的「暴力爆破」情况；保留 CRC 爆破 / 手动模式。

## [0.2.2] - 2026-07-02

### 新增

- 8 个第三方图片隐写工具：cloacked-pixel、ImageMask（文本/文件）、BMP 填补字节、Invoke-PSImage、stegpy、BrainTools、PixelJihad。
- 哈希爆破、通用口令爆破（字典驱动任意节点）、压缩包密码爆破模板。
- HTTP 请求（GET/POST…、HTTP 1.1/2.0、UA/Cookie）、字符串匹配、压缩包文件列表 三个节点。

### 改进

- 运行记录按类型展示节点实际输出（图片 / 字节 / 字符串）。
- 模块库可拖动调宽、分类重排、每个模块加说明；流程模板新增「隐写术」分类与多个常见流程。
- 打包改用 UPX `--lzma --best` 压缩，Release 工作流自动生成更新日志。

## [0.2.1] - 2026-07-02

- 新增 imageIN(图影) 图片藏文件 提取/嵌入，以及配套的 imageIN 提取流程模板。
- 流程模板界面重做：按分类分组、卡片紧凑化。

## [0.2.0] - 2026-07-02

### 新增 · 内嵌 MCP 服务（供 AI 客户端调用）

- 在应用内内嵌 **MCP（Model Context Protocol）服务器**，让 Claude Code / Cursor / Codex / Claude Desktop 等 AI 客户端直接驱动节点图引擎。
  - 基于 **rmcp 2.0 + axum** 的 streamable-HTTP 服务，绑定 `127.0.0.1`，Bearer 令牌鉴权，运行在独立 tokio 线程。
  - **默认关闭**，在「设置 → MCP 服务」中一键启停；配置持久化到 `mcp.json`，令牌首次启用时自动生成。
- **22 个通用元工具**：
  - 发现：`list_categories`、`list_nodes`、`describe_node`、`list_modules`
  - 执行：`run_node`、`run_graph`（不带参数即运行当前画布）
  - 画布协作：`get_canvas`、`set_canvas`、`add_node`、`connect`、`set_param`、`remove_node`、`remove_edge`、`move_node`
  - 持久化与 AI：`save_workflow`、`load_workflow`、`save_composite_module`、`save_script_module`、`generate_workflow`
  - 其他：`get_settings`（API Key 脱敏）、`detect_tool`、`ping`
- **实时画布协作**：前后端画布双向同步，AI 的增删改即时显示在用户画布上（`sync_canvas` ↔ `mcp://canvas-update`，`applyingRemote` 标志 + `rev` 计数双重回声环守卫）。
- **渐进式节点发现（省 token）**：先 `list_categories` 看分类，再按分类精简列出（默认仅 id+名称），用到某节点才取参数详情；典型发现开销从约 78 KB 降到约 2 KB。
- **AI 友好的 IO 适配**：bytes/image 输出落盘并返回文件路径、长文本截断，避免淹没上下文；输入支持 base64 或文件路径。

### 变更

- 设置界面重构为**分类导航**（AI 模型 / 输出目录 / 外部工具 / MCP 服务，四个 tab），不再单页长滚动。
- MCP 面板内置 **Claude Code / Cursor / Codex / 其他客户端**的一键复制配置片段（自动填入端点与令牌）。

### 安全

- MCP 服务默认关闭、仅监听本机、强制 Bearer 令牌；`get_settings` 对 AI 隐藏 API Key。

### 其他

- `mcp` 作为默认 Cargo feature（`--no-default-features` 可移除整套 MCP 依赖）。
- 服务器逻辑经 `AppBridge` trait 抽象，可脱离 Tauri 单测；新增 HTTP 端到端集成测试。
- 版本号 0.1.0 → 0.2.0。

## [0.1.0]

- 初始版本：基于节点图的可视化分析工作台 —— 约 116 个内置节点、可视化画布、自定义复合/脚本节点、AI 生成工作流、流程文件（.lml）等。

[0.2.3]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.3
[0.2.2]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.2
[0.2.1]: https://github.com/Tokeii0/LovelyMiscLab/releases/tag/v0.2.1
[0.2.0]: https://github.com/Tokeii0/lovelymisclab/releases/tag/v0.2.0
