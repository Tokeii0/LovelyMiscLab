# 更新日志

本项目所有值得注意的变更都会记录在此文件。版本遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

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

[0.2.0]: https://github.com/Tokeii0/lovelymisclab/releases/tag/v0.2.0
