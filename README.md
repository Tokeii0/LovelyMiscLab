<div align="center">

<br />

<img src="assets/banner.svg" alt="LovelyMiscLab" width="800" />

<br />
<br />

面向取证的节点式可视化工作台 —— 像 ComfyUI 一样搭工作流，专为编码解密、隐写取证、图像分析、密码学而生。

<br />

![Version](https://img.shields.io/badge/version-0.2.9-8b5cf6)
![Platforms](https://img.shields.io/badge/Windows_·_Linux_·_macOS-portable-2ea44f)
![Nodes](https://img.shields.io/badge/内置节点-270-8b5cf6)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white)
![MCP](https://img.shields.io/badge/MCP-rmcp_2.0-000000?logo=anthropic&logoColor=white)
![QQ群](https://img.shields.io/badge/QQ群-668600249-0099ff?logo=qq&logoColor=white)
![License](https://img.shields.io/badge/License-GPLv3-blue)

<br />

[主要特性](#主要特性) · [MCP 与 AI 调用](#mcp-与-ai-调用) · [工作区](#工作区) · [节点分类](#节点分类) · [架构](#架构) · [下载](#下载--安装) · [贡献](#贡献)

<br />

</div>

---

> [!NOTE]
> **✨ 0.2.9 新特性** —— 常见 CTF 小技巧补齐 · 二进制分析（ELF / PE / Mach-O / DEX / PE 深度检查）· 文件格式与文档/APK 取证 · 古典密码自动分析/爆破 · 可视化分析（字节直方图 / 熵曲线 / 字节分布图）· 控制/逻辑闭环 · 270 个内置节点。

## 这是什么

LovelyMiscLab 是一个桌面应用，核心是一个**类型化的节点图引擎（typed node-graph engine）**，专为编码解密、隐写取证、图像分析、密码学等场景打造。

设计上**做的是类似 ComfyUI 的节点式工作流**——你在画布上拖拽「节点」（Base64 解码、XOR、LSB 隐写提取、二维码解码、AES 解密、图像滤镜……），用连线把它们组成一条数据流 DAG，后端按拓扑序执行，并把每个节点的结果实时回显到画布上。同样是数据驱动、可视化编排、节点自由扩展，只是把「生成图像」换成了「Misc 分析」。

它可以当作：

- 🧰 **快捷工具箱** —— 单节点执行，把任意一个节点当独立小工具用
- 🔗 **流程编排器** —— 整图执行，带内容寻址的增量缓存（改哪算哪的「实时模式」）
- 🤖 **AI 工作台** —— 自然语言让 **AI Agent** 一步步在画布上搭流程，或让外部 AI 客户端**经 MCP 直接调用引擎、与你的画布实时协作**
- 🧱 **可扩展平台** —— 把选中子图封装为「复合模块」，或把外部脚本 / 程序接入为本地自定义节点
- 🎯 **出题工作台** —— 用 LSB 嵌入、ZIP 创建 / 伪加密等「出题」节点，反向造一道 misc 题

<img width="1920" height="1080" alt="canvas" src="https://github.com/user-attachments/assets/7af12c9e-05dd-4e87-ad4d-31f0fe836df6" />
<img width="1920" height="1080" alt="workflow" src="https://github.com/user-attachments/assets/ea5ddf9d-e176-4527-89a3-0ee942943567" />

## 主要特性

- 🧩 **270 个内置节点 · 16 大类** —— 从 Base 家族、经典/现代密码、哈希，到隐写、图像/音频/二进制分析、文件格式/文档/APK 取证、压缩包解包与「出题」节点，开箱即用（[分类见下](#节点分类)）。
- 🔌 **类型化端口** —— 连线携带强类型值（Text / Number / Bool / Json / Bytes / Image / Fingerprint…），连接时自动做类型校验与必要的隐式转换。
- 🤖 **AI Agent 建图** —— 一句话描述任务，AI **逐步**在画布上加节点 / 连线 / 设参、运行中间结果并据此自适应，你实时看着流程长出来；模型不支持工具调用时自动回退一次性生成。
- 🪄 **端口 AI 预测** —— 悬停任意输入 / 输出端口弹出 ✨，推荐类型兼容的下一个节点（离线可用）并支持手动筛选，确认即建即连、自动避让不重叠。
- 🧹 **一键整理节点** —— 右键「整理节点」，按当前视口宽高比把图竖向优先打包成多列并缩放适配，整张图一屏可见。
- 🛰️ **MCP 服务** —— 内嵌 MCP 服务器，让 Claude Code / Cursor / Codex 等 AI 客户端**直接驱动引擎、读写你正在编辑的画布**（[详见下节](#mcp-与-ai-调用)）。
- ⚡ **增量执行** —— 内容寻址缓存，改动后只重算受影响的节点（「实时模式」）。
- 🧱 **自定义扩展** —— 复合模块（子图打包成一个节点，防递归深度 16 层）+ 脚本节点（外部进程接入，支持 stdin / argv / 临时文件三种投递方式）。
- 🔒 **本地优先** —— 所有计算在本机完成，数据不出设备。

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

> [!TIP]
> 画布内的 **AI Agent** 与 **端口 AI 预测** 是应用**自带**的功能（用你在「设置 → AI 模型」里配置的 OpenAI 兼容模型），开箱即用、无需 MCP；MCP 则是给**外部** AI 客户端接管引擎用的另一条通道。

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

> [!WARNING]
> **安全**：默认关闭、仅监听本机、强制 Bearer 令牌；`get_settings` 对 AI 隐藏 API Key。启用即意味着持令牌的 AI 可运行节点/脚本、读写画布，请勿泄露令牌。

## 工作区

- **命令面板**（`Ctrl / Cmd + K`）—— 快速跳转视图、执行动作
- **整理节点**（画布右键）—— 一键把图打包排布到当前视口，全部可见
- **运行历史与运行到节点** —— 回看每次执行、只跑到某个节点为止
- **撤销 / 重做** —— 画布编辑全程可回退（AI Agent 的整次搭建折叠为一步）
- **自动保存与最近项目** —— 断电不丢，快速重开
- **帮助面板与边上数据预览** —— 连线上直接看流过的值
- **AI 解释 / 修复工作流** —— 让 LLM 解读或修补一张图

## 节点分类

> 共 **270** 个内置节点，按分类：

| 分类 | 数量 | 举例 |
|---|:--:|---|
| 编码/加密 | 46 | Base32/45/58/62/64/85/92 · Hex · URL · 二维码 · 摩斯 · 培根 · 盲文 · Brainfuck · DNA · 重复 XOR 破解 |
| 图像处理 | 38 | 通道/混合/滤镜/几何/色彩空间/差分 · 频域(FFT) · GIF · PNG/JPG/BMP 宽高修复 · 盲水印 · 01↔图像 |
| 加密解密 | 39 | AES/DES/Blowfish/RC4/ChaCha/Salsa/RSA · 维吉尼亚/仿射/Atbash/Playfair/Enigma/ADFGVX/栅栏 · 古典密码自动分析/爆破 · 数论/RSA 小工具 · PGP |
| 隐写术 | 29 | 零宽字符 · StegCloak · 空白/SNOW · LSB 提取/嵌入 · zsteg 扫描 · imageIN/ImageMask/stegpy/PixelJihad |
| 控制/逻辑 | 22 | 开关 · 门控 · 迭代 · 映射/过滤/归约 · 正则路由 · 合流 · 数值比较 · 列表工具 |
| 工具/分析 | 25 | 熵值 · 字频 · hexdump · EXIF · 文件类型识别 · PNG/JPEG/GIF/PDF 结构 · OOXML/APK 取证 · 文件雕刻 · HTTP · JWT · IP 转换 |
| 文本处理 | 14 | 拼接/分割/替换/正则/大小写/去重/排序… |
| 进制转换 | 9 | 二/八/十/十六 · 字符码 · 十进制 |
| 压缩包 | 9 | ZIP/7z/TAR/GZIP/RAR 解包 · ZIP 列表/目录差异/CRC 爆破/bkcrack · **ZIP 创建** · **ZIP 伪加密（出题）** |
| 输入输出 | 7 | 文本输入/输出/展示 · 文件导入/输出 · 图片输入/查看 |
| 二进制分析 | 13 | ELF / PE / Mach-O 信息 · DEX 字符串 · 节区列表/提取/熵 · PE 资源/证书/imphash/壳提示/.NET 元数据 · strings · overlay |
| 音频处理 | 5 | WAV 信息 · 频谱图 · WAV LSB · DTMF · DeepSound |
| 字符编码 | 5 | GBK/UTF/Big5 等字符集互转 · Quoted-Printable · 全角/半角 |
| 哈希/摘要 | 4 | MD5/SHA/SHA3/RIPEMD/BLAKE2/Whirlpool/SM3/CRC32/HMAC/bcrypt |
| 可视化分析 | 3 | 字节直方图 · 熵曲线 · 字节分布图 |
| AI | 2 | 文本判断 `ai_judge` · 视觉识图 `ai_vision` |

## 架构

采用 Cargo workspace，把与 Tauri 无关的分析引擎和薄适配层彻底分离：

```
LovelyMiscLab/
├── crates/misclab-core/     # 纯 Rust 分析引擎（可无头单测、可被 CLI 复用）
│   ├── src/graph/           # 图引擎：端口类型、模型、执行器、缓存、复合/脚本节点
│   ├── src/node/            # Node trait、NodeDescriptor（数据驱动 UI 的单一真相源）、注册表
│   ├── src/nodes/           # 270 个内置节点实现（一文件一节点或一对编解码）
│   ├── src/ai.rs            # OpenAI 兼容的 chat / vision / 工具调用(chat_step)
│   └── tests/               # 集成测试（大量编码解密对照用例）
├── src-tauri/               # Tauri 应用外壳（薄适配器，over misclab-core）
│   ├── src/mcp/             # 内嵌 MCP 服务器（rmcp/axum）：工具、状态、鉴权、IO 适配、画布桥接
│   ├── src/commands/        # 暴露的 Tauri command（图执行 / AI Agent / 项目 / 模块 / 设置 / MCP…）
│   ├── src/db.rs            # SQLite 持久化
│   └── src/lib.rs           # 应用入口：注册插件、构建注册表、（可选）自启 MCP
├── src/                     # React 前端
│   ├── flow/                # 画布、节点、连线、检查器、整理布局、端口建议、Agent 回放、canvasSync
│   ├── app/                 # 标题栏、命令面板、自动保存、运行控制台、Agent 面板、各类对话框
│   ├── views/               # 画布 / 模块 / 模板 / 设置（含 MCP 面板）等视图
│   ├── store/               # zustand 状态（graph / run / agent / descriptors…）
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

`NodeDescriptor` 是**单一真相源**：后端声明节点的输入 / 输出 / 参数控件，前端完全据此渲染 UI，无需为每个节点写前端代码。AI Agent 与 MCP 服务器复用同一套引擎与执行器；`AppBridge` trait 让 MCP 能脱离 Tauri 单测。

### 关键 Tauri Commands

| 分类 | Command |
|---|---|
| 图执行 | `list_node_descriptors` · `run_node` · `run_graph` · `cancel_job` · `reset_run` |
| AI | `agent_run`（分步 Agent，流式）· `suggest_next_nodes`（端口预测）· `generate_workflow` · `explain_workflow` |
| MCP | `mcp_start` · `mcp_stop` · `mcp_status` · `mcp_get_config` · `mcp_set_config` · `sync_canvas` |
| 复合/脚本模块 | `list/save/delete_composite_module` · `list/save/delete_script_module` |
| 工程与设置 | `save_project` · `load_project` · `get_settings` · `set_settings` · `detect_tool` |

## 技术栈

**前端**：React 19 · TypeScript · Vite 7 · Tailwind CSS v4 · @xyflow/react（节点图）· zustand · TanStack Query

**后端**：Rust 2021 · Tauri 2 · petgraph（图算法）· rusqlite（SQLite，bundled）· image / imageproc · rustfft · RustCrypto 全家桶 · rPGP · encoding_rs

**MCP**：[rmcp](https://github.com/modelcontextprotocol/rust-sdk) 2.0 · axum · tokio（`mcp` feature，默认开启）

## 下载 / 安装

到 [Releases](https://github.com/Tokeii0/LovelyMiscLab/releases) 下载对应平台的绿色版（免安装）：

| 平台 | 资源 | 说明 |
|---|---|---|
| 🪟 **Windows** | `*-windows-x64.exe` | 双击运行。依赖系统自带的 WebView2 运行时（Win10/11 已内置），已用 UPX 压缩。 |
| 🐧 **Linux** | `*-linux-x64` | `chmod +x` 后运行。需要 WebKitGTK 4.1（Debian/Ubuntu：`sudo apt install libwebkit2gtk-4.1-0`）。 |
| 🍎 **macOS** | `*-macos-universal.dmg` | Intel + Apple Silicon 通用；拖入「应用程序」。未签名，首次打开若被 Gatekeeper 拦截：右键 →「打开」，或终端执行 `xattr -cr /Applications/LovelyMiscLab.app`。 |

> 应用内置自动更新：新版本发布后会在应用内提示升级（macOS 因 `.dmg` 内是 `.app` 包，改为引导到发布页手动下载）。

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
cargo clippy --workspace --all-targets -- -D warnings   # 零告警门禁（同 CI）
cargo test -p misclab-core         # 引擎单元/集成测试（无需 Tauri）
cargo test -p misclab-app --features mcp   # 应用层 + MCP 测试
cargo build --no-default-features  # 不含 MCP 栈的精简构建
```

`crates/misclab-core` 完全独立于 Tauri，可单独测试与复用；MCP 由 `mcp` Cargo feature 门控（默认开启，服务本身运行时默认关闭）。CI 在 Windows / Linux / macOS 三系统上跑构建、测试与 Clippy 零告警门禁。

## 贡献

欢迎贡献新节点、修 bug、完善文档。**新增节点请用 Rust 实现，且仓库只允许使用 Rust 版依赖库**——想移植某个 Python 工具，请先用 Rust 重写。完整规范、节点开发模板、PR 流程见 **[开发手册 CONTRIBUTING.md](./CONTRIBUTING.md)**。

## 推荐 IDE

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 许可

[GPL-3.0](./LICENSE) · 详见 [CHANGELOG](./CHANGELOG.md)
