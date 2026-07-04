<div align="center">

<br />

<img src="assets/banner.svg" alt="LovelyMiscLab" width="800" />

<br />
<br />

面向取证的节点式可视化工作台 —— 像 ComfyUI 一样搭工作流，专为编码解密、隐写取证、图像分析、密码学而生。

<br />

![Version](https://img.shields.io/badge/version-0.2.6-8b5cf6)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)
![MCP](https://img.shields.io/badge/MCP-rmcp_2.0-000000?logo=anthropic&logoColor=white)
![QQ群](https://img.shields.io/badge/QQ群-668600249-0099ff?logo=qq&logoColor=white)
![License](https://img.shields.io/badge/License-GPLv3-blue)

<br />

[主要特性](#主要特性) · [MCP 与 AI 调用](#mcp-与-ai-调用) · [工作区](#工作区) · [节点分类](#节点分类) · [架构](#架构) · [贡献](#贡献)

<br />

</div>

---

## 这是什么

LovelyMiscLab 是一个桌面应用，核心是一个**类型化的节点图引擎（typed node-graph engine）**，专为编码解密、隐写取证、图像分析、密码学等场景打造。

设计上**做的是类似 ComfyUI 的节点式工作流**——你在画布上拖拽「节点」（Base64 解码、XOR、LSB 隐写提取、二维码解码、AES 解密、图像滤镜……），用连线把它们组成一条数据流 DAG，后端按拓扑序执行，并把每个节点的结果实时回显到画布上。同样是数据驱动、可视化编排、节点自由扩展，只是把「生成图像」换成了「Misc 分析」。

它可以当作：

- **快捷工具箱** —— 单节点执行，把任意一个节点当独立小工具用
- **流程编排器** —— 整图执行，带内容寻址的增量缓存（改哪算哪的「实时模式」）
- **AI 工作台** —— 自然语言一键生成流程，或让外部 AI 客户端**经 MCP 直接调用引擎、与你的画布实时协作**
- **可扩展平台** —— 把选中子图封装为「复合模块」，或把外部脚本 / 程序接入为本地自定义节点

<img width="1920" height="1080" alt="canvas" src="https://github.com/user-attachments/assets/7af12c9e-05dd-4e87-ad4d-31f0fe836df6" />
<img width="1920" height="1080" alt="workflow" src="https://github.com/user-attachments/assets/ea5ddf9d-e176-4527-89a3-0ee942943567" />

## 主要特性

- **167 个内置节点**，覆盖 13 个分类（[见下表](#节点分类)）——从 Base 家族、经典密码、现代对称/非对称加密，到隐写、图像频域分析、压缩包解包，开箱即用。
- **类型化端口** —— 连线携带强类型值（Text / Number / Bool / Json / Bytes / Image / Fingerprint…），连接时自动做类型校验与必要的隐式转换。
- **增量执行** —— 内容寻址缓存，改动后只重算受影响的节点（「实时模式」）。
- **MCP 服务** —— 内嵌 MCP 服务器，让 Claude Code / Cursor / Codex 等 AI 客户端**直接驱动引擎、读写你正在编辑的画布**（[详见下节](#mcp-与-ai-调用)）。
- **AI 生成与解释流程** —— 自然语言 → 校验过的真实节点图（自动分层布局）；也能让 AI 解释一张图为何这么连、哪里有问题。
- **完整工作区** —— 命令面板、运行历史、撤销/重做、自动保存与最近项目、边上数据预览……（[详见](#工作区)）。
- **自定义扩展** —— 复合模块（子图打包成一个节点，防递归深度 16 层）+ 脚本节点（外部进程接入，支持 stdin / argv / 临时文件三种投递方式）。
- **本地优先** —— 所有计算在本机完成，数据不出设备。

## MCP 与 AI 调用

**v0.2.0 起**，应用内嵌了一个 **MCP（[Model Context Protocol](https://modelcontextprotocol.io/)）服务器**：外部 AI 客户端可以直接发现节点、运行单节点或整条流水线，并**与你当前正在编辑的画布实时协作**——AI 增删改的节点会即时出现在你的屏幕上。

- 基于 **rmcp 2.0 + axum** 的 streamable-HTTP 服务，绑定 `127.0.0.1`、Bearer 令牌鉴权、跑在独立线程。
- **默认关闭**，在 **设置 → MCP 服务** 一键启停；令牌首次启用时自动生成，面板内置各客户端的一键复制配置。
- 22 个通用元工具，按**渐进式发现**设计以节省 token：

| 类别 | 工具 |
|---|---|
| 发现 | `list_categories` → `list_nodes`（精简）→ `describe_node` · `list_modules` |
| 执行 | `run_node` · `run_graph`（不带参数即运行当前画布） |
| 画布 | `get_canvas` · `set_canvas` · `add_node` · `connect` · `set_param` · `remove_node` · `move_node`… |
| 持久化与 AI | `save_workflow` · `load_workflow` · `save_composite_module` · `generate_workflow` |
| 其他 | `get_settings`（脱敏）· `detect_tool` · `ping` |

<details>
<summary><b>在 AI 客户端中连接（点击展开）</b></summary>

启用后复制面板里的端点与令牌，然后：

**Claude Code**
```bash
claude mcp add --transport http lovelymisclab http://127.0.0.1:8765/mcp \
  --header "Authorization: Bearer <令牌>"
```

**Cursor** —— `~/.cursor/mcp.json`
```json
{ "mcpServers": { "lovelymisclab": {
  "url": "http://127.0.0.1:8765/mcp",
  "headers": { "Authorization": "Bearer <令牌>" }
} } }
```

**Codex / 其他仅 stdio 的客户端** —— 经 `mcp-remote` 桥接
```toml
[mcp_servers.lovelymisclab]
command = "npx"
args = ["-y", "mcp-remote", "http://127.0.0.1:8765/mcp", "--header", "Authorization: Bearer <令牌>"]
```
</details>

> **安全**：默认关闭、仅监听本机、强制 Bearer 令牌；`get_settings` 对 AI 隐藏 API Key。启用即意味着持令牌的 AI 可运行节点/脚本、读写画布，请勿泄露令牌。

## 工作区

- **命令面板**（`Ctrl / Cmd + K`）—— 快速跳转视图、执行动作
- **运行历史与运行到节点** —— 回看每次执行、只跑到某个节点为止
- **撤销 / 重做** —— 画布编辑全程可回退
- **自动保存与最近项目** —— 断电不丢，快速重开
- **帮助面板与边上数据预览** —— 连线上直接看流过的值
- **AI 解释 / 修复工作流** —— 让 LLM 解读或修补一张图

## 节点分类

> 共 **167** 个内置节点，按分类：

| 分类 | 数量 | 举例 |
|---|:--:|---|
| 编码/加密 | 40 | Base32/45/58/62/64/85/92 · Hex · URL · ROT13 · 摩斯 · 培根 · 盲文 · A1Z26 |
| 图像处理 | 34 | 通道/混合/滤镜/几何/色彩空间/差分 · 频域(FFT) · GIF · PNG 宽高修复 · 盲水印 · 01↔图像 |
| 加密解密 | 25 | AES/DES/Blowfish/RC4/ChaCha/Salsa/RSA · 维吉尼亚/仿射/Atbash/Playfair/Enigma/ADFGVX/栅栏 · PGP · JWT |
| 文本处理 | 14 | 拼接/分割/替换/正则/大小写/去重/排序… |
| 工具/分析 | 11 | 熵值 · 字频 · hexdump · EXIF · 文件类型识别 · 比较 |
| 控制/逻辑 | 11 | 开关 · 门控 · 迭代 · 过滤 · 选择器 |
| 进制转换 | 9 | 二/八/十/十六 · 字符码 · 十进制 |
| 隐写术 | 7 | 零宽字符（含自动识别）· StegCloak · 空白/SNOW · LSB 图像隐写 |
| 输入输出 | 5 | 文本输入/输出 · 文件导入 · 图片输入 · 文件输出 |
| 字符编码 | 4 | GBK/UTF 等字符集互转 |
| 哈希/摘要 | 3 | MD5/SHA/SHA3/RIPEMD/BLAKE2/Whirlpool/SM3/CRC32/HMAC/bcrypt |
| AI | 2 | 文本判断 `ai_judge` · 视觉识图 `ai_vision` |
| 压缩包 | 2 | ZIP/7z/TAR/GZIP/RAR 解包 |

## 架构

采用 Cargo workspace，把与 Tauri 无关的分析引擎和薄适配层彻底分离：

```
LovelyMiscLab/
├── crates/misclab-core/     # 纯 Rust 分析引擎（可无头单测、可被 CLI 复用）
│   ├── src/graph/           # 图引擎：端口类型、模型、执行器、缓存、复合/脚本节点
│   ├── src/node/            # Node trait、NodeDescriptor（数据驱动 UI 的单一真相源）、注册表
│   ├── src/nodes/           # 167 个内置节点实现（一文件一节点或一对编解码）
│   ├── src/ai.rs            # OpenAI 兼容的 chat / vision 调用
│   └── tests/               # 集成测试（大量编码解密对照用例）
├── src-tauri/               # Tauri 应用外壳（薄适配器，over misclab-core）
│   ├── src/mcp/             # 内嵌 MCP 服务器（rmcp/axum）：工具、状态、鉴权、IO 适配、画布桥接
│   ├── src/commands/        # 暴露的 Tauri command（图执行 / 项目 / 模块 / 设置 / MCP…）
│   ├── src/db.rs            # SQLite 持久化
│   └── src/lib.rs           # 应用入口：注册插件、构建注册表、（可选）自启 MCP
├── src/                     # React 前端
│   ├── flow/                # 画布、节点、连线、检查器、canvasSync（MCP 画布双向同步）
│   ├── app/                 # 标题栏、命令面板、自动保存、运行控制台、各类对话框
│   ├── views/               # 画布 / 模块 / 模板 / 设置（含 MCP 面板）等视图
│   ├── store/               # zustand 状态（graph / run / workspace / descriptors…）
│   └── lib/                 # Tauri IPC 绑定、类型、工程存取
└── scripts/                 # UTF-8 检查等辅助脚本
```

### 数据流

```
前端画布 (React + @xyflow/react)
        │  SerializedGraph (JSON)                        ┌──────────────────────┐
        ▼                                                │  外部 AI 客户端        │
Tauri commands (src-tauri) ──Channel──▶ 流式进度/每节点结果 │  Claude Code / Cursor │
        │                                                └──────────┬───────────┘
        ▼                                          HTTP / Bearer     │  MCP
GraphExecutor (misclab-core) 拓扑排序→逐节点 run→增量缓存 ◀── src/mcp ─┘  画布双向同步
        │
        ▼
NodeRegistry + NodeDescriptor ──▶ 前端据此渲染调色板/节点体/连接校验
```

`NodeDescriptor` 是**单一真相源**：后端声明节点的输入 / 输出 / 参数控件，前端完全据此渲染 UI，无需为每个节点写前端代码。MCP 服务器复用同一套引擎，`AppBridge` trait 让它能脱离 Tauri 单测。

### 关键 Tauri Commands

| 分类 | Command |
|---|---|
| 图执行 | `list_node_descriptors` · `run_node` · `run_graph` · `cancel_job` · `reset_run` |
| AI | `generate_workflow` · `explain_workflow` |
| MCP | `mcp_start` · `mcp_stop` · `mcp_status` · `mcp_get_config` · `mcp_set_config` · `sync_canvas` |
| 复合/脚本模块 | `list/save/delete_composite_module` · `list/save/delete_script_module` |
| 工程与设置 | `save_project` · `load_project` · `get_settings` · `set_settings` · `detect_tool` |

## 技术栈

**前端**：React 19 · TypeScript · Vite 7 · Tailwind CSS v4 · @xyflow/react（节点图）· zustand · TanStack Query

**后端**：Rust 2021 · Tauri 2 · petgraph（图算法）· rusqlite（SQLite，bundled）· image / imageproc · rustfft · RustCrypto 全家桶 · rPGP · encoding_rs

**MCP**：[rmcp](https://github.com/modelcontextprotocol/rust-sdk) 2.0 · axum · tokio（`mcp` feature，默认开启）

## 下载 / 安装

到 [Releases](https://github.com/Tokeii0/LovelyMiscLab/releases) 下载对应平台的绿色版（免安装）：

- **Windows** —— `LovelyMiscLab-*-windows-x64.exe`，双击运行。依赖系统自带的 WebView2 运行时（Win10/11 已内置），已用 UPX 压缩。
- **Linux** —— `LovelyMiscLab-*-linux-x64`，`chmod +x` 后运行。需要 WebKitGTK 4.1（Debian/Ubuntu：`sudo apt install libwebkit2gtk-4.1-0`）。
- **macOS** —— `LovelyMiscLab-*-macos-universal.dmg`（Intel + Apple Silicon 通用），打开后拖入「应用程序」。未签名，首次打开若被 Gatekeeper 拦截：右键 →「打开」，或终端执行 `xattr -cr /Applications/LovelyMiscLab.app`。

## 开发

### 前置依赖

- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/) · [Rust 工具链](https://www.rust-lang.org/tools/install) · [Tauri 2 系统依赖](https://tauri.app/start/prerequisites/)

### 常用命令

```bash
pnpm install                       # 安装前端依赖
pnpm tauri dev                     # 桌面开发模式（Tauri + Vite 热更新）
pnpm dev                           # 仅浏览器预览前端（mock 节点，无 Tauri IPC）
pnpm tauri build                   # 构建发布版桌面应用

pnpm check:utf8                    # 校验源码 UTF-8 编码
cargo test -p misclab-core         # 引擎单元/集成测试（无需 Tauri）
cargo test -p misclab-app --features mcp   # 应用层 + MCP 测试
cargo build --no-default-features  # 不含 MCP 栈的精简构建
```

`crates/misclab-core` 完全独立于 Tauri，可单独测试与复用；MCP 由 `mcp` Cargo feature 门控（默认开启，服务本身运行时默认关闭）。

## 贡献

欢迎贡献新节点、修 bug、完善文档。**新增节点请用 Rust 实现，且仓库只允许使用 Rust 版依赖库**——想移植某个 Python 工具，请先用 Rust 重写。完整规范、节点开发模板、PR 流程见 **[开发手册 CONTRIBUTING.md](./CONTRIBUTING.md)**。

## 推荐 IDE

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 许可

[GPL-3.0](./LICENSE) · 详见 [CHANGELOG](./CHANGELOG.md)
